use std::marker::PhantomData;


pub struct Binding<T> {
    bind_group: wgpu::BindGroup,
    _marker: PhantomData<T>,
}

pub trait Bind {
    fn bind<T>(data: &T) -> Binding<T>;
}
