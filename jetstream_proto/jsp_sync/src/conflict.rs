use crate::crdt::LWWRegister;

pub trait ConflictResolver<T> {
    fn resolve(&self, local: &LWWRegister<T>, remote: &LWWRegister<T>) -> LWWRegister<T>;
}

pub struct LWWResolver;

impl<T: Clone> ConflictResolver<T> for LWWResolver {
    fn resolve(&self, local: &LWWRegister<T>, remote: &LWWRegister<T>) -> LWWRegister<T> {
        let mut result = local.clone();
        result.merge(remote.clone());
        result
    }
}

pub struct ManualResolver<F> 
where F: Fn(&LWWRegister<String>, &LWWRegister<String>) -> LWWRegister<String>
{
    resolve_fn: F,
}

impl<F> ManualResolver<F> 
where F: Fn(&LWWRegister<String>, &LWWRegister<String>) -> LWWRegister<String>
{
    pub fn new(resolve_fn: F) -> Self {
        Self { resolve_fn }
    }
}

impl<F> ConflictResolver<String> for ManualResolver<F> 
where F: Fn(&LWWRegister<String>, &LWWRegister<String>) -> LWWRegister<String>
{
    fn resolve(&self, local: &LWWRegister<String>, remote: &LWWRegister<String>) -> LWWRegister<String> {
        (self.resolve_fn)(local, remote)
    }
}
