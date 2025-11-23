use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::Result;
use socket2::{Socket, Domain, Type, Protocol};

/// Optimized UDP transport with socket options for maximum performance
#[derive(Clone)]
pub struct UdpTransport {
    socket: Arc<UdpSocket>,
}

impl UdpTransport {
    /// Create a new UDP transport with optimized socket settings
    pub async fn bind(addr: &str) -> Result<Self> {
        let addr: SocketAddr = addr.parse()?;
        
        // Create socket with socket2 for low-level options
        let socket = Socket::new(
            if addr.is_ipv4() { Domain::IPV4 } else { Domain::IPV6 },
            Type::DGRAM,
            Some(Protocol::UDP),
        )?;
        
        // Apply socket optimizations
        Self::configure_socket(&socket)?;
        
        // Bind socket
        socket.bind(&addr.into())?;
        socket.set_nonblocking(true)?;
        
        // Convert to tokio UdpSocket
        let std_socket: std::net::UdpSocket = socket.into();
        let tokio_socket = UdpSocket::from_std(std_socket)?;
        
        tracing::info!(
            addr = %addr,
            "UDP transport created with optimizations: SO_REUSEPORT, 4MB buffers"
        );
        
        Ok(Self {
            socket: Arc::new(tokio_socket),
        })
    }
    
    /// Configure socket with performance optimizations
    fn configure_socket(socket: &Socket) -> Result<()> {
        // SO_REUSEADDR - allow address reuse
        socket.set_reuse_address(true)?;
        
        // Platform-specific optimizations
        #[cfg(unix)]
        {
            use socket2::SockRef;
            let sock_ref = SockRef::from(socket);
            
            // SO_REUSEPORT - enable multi-threaded accept/recv (Linux/BSD)
            #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
            {
                sock_ref.set_reuse_port(true)?;
                tracing::debug!("SO_REUSEPORT enabled");
            }
            
            // IP_MTU_DISCOVER - enable path MTU discovery (Linux only)
            #[cfg(target_os = "linux")]
            {
                use std::os::unix::io::AsRawFd;
                unsafe {
                    let fd = socket.as_raw_fd();
                    let val: libc::c_int = libc::IP_PMTUDISC_DO;
                    libc::setsockopt(
                        fd,
                        libc::IPPROTO_IP,
                        libc::IP_MTU_DISCOVER,
                        &val as *const _ as *const libc::c_void,
                        std::mem::size_of_val(&val) as libc::socklen_t,
                    );
                    tracing::debug!("IP_MTU_DISCOVER enabled");
                }
            }
            
            // IP_RECVERR - receive ICMP errors (Linux only)
            #[cfg(target_os = "linux")]
            {
                use std::os::unix::io::AsRawFd;
                unsafe {
                    let fd = socket.as_raw_fd();
                    let val: libc::c_int = 1;
                    libc::setsockopt(
                        fd,
                        libc::IPPROTO_IP,
                        libc::IP_RECVERR,
                        &val as *const _ as *const libc::c_void,
                        std::mem::size_of_val(&val) as libc::socklen_t,
                    );
                    tracing::debug!("IP_RECVERR enabled");
                }
            }
        }
        
        // SO_RCVBUF - 4MB receive buffer
        const BUFFER_SIZE: usize = 4 * 1024 * 1024; // 4MB
        socket.set_recv_buffer_size(BUFFER_SIZE)?;
        tracing::debug!("SO_RCVBUF set to {}MB", BUFFER_SIZE / 1024 / 1024);
        
        // SO_SNDBUF - 4MB send buffer
        socket.set_send_buffer_size(BUFFER_SIZE)?;
        tracing::debug!("SO_SNDBUF set to {}MB", BUFFER_SIZE / 1024 / 1024);
        
        Ok(())
    }

    pub async fn send_to(&self, data: &[u8], addr: SocketAddr) -> Result<usize> {
        let len = self.socket.send_to(data, addr).await?;
        Ok(len)
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let (len, addr) = self.socket.recv_from(buf).await?;
        Ok((len, addr))
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }
}
