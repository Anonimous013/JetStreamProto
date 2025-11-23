use jsp_gateway::proxy::Proxy;
use jsp_gateway::balancer::{LoadBalancer, Strategy};
use tokio::net::UdpSocket;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_gateway_proxy() {
    // 1. Start Backend Server
    let backend_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let backend_addr = backend_socket.local_addr().unwrap();
    
    tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        loop {
            let (len, src) = backend_socket.recv_from(&mut buf).await.unwrap();
            // Echo back
            backend_socket.send_to(&buf[..len], src).await.unwrap();
        }
    });

    // 2. Start Gateway
    let balancer = Arc::new(LoadBalancer::new(vec![backend_addr], Strategy::RoundRobin));
    // Bind to random port
    let proxy = Proxy::new("127.0.0.1:0", balancer).await.unwrap();
    let gateway_addr = proxy.socket.local_addr().unwrap(); // Access socket via pub field? No, need to make it pub or add accessor
    
    // We need to run proxy in background
    let proxy_arc = Arc::new(proxy);
    let proxy_task = proxy_arc.clone();
    tokio::spawn(async move {
        proxy_task.run().await.unwrap();
    });
    
    // 3. Start Client
    let client_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client_socket.connect(gateway_addr).await.unwrap();
    
    // Send data
    let msg = b"hello gateway";
    client_socket.send(msg).await.unwrap();
    
    // Receive echo
    let mut buf = [0u8; 1024];
    let len = tokio::time::timeout(Duration::from_secs(1), client_socket.recv(&mut buf))
        .await
        .expect("Timeout waiting for response")
        .unwrap();
        
    assert_eq!(&buf[..len], msg);
}
