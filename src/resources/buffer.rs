use wgpu::util::{BufferInitDescriptor, DeviceExt};

pub struct CpuBuffer<T: bytemuck::Pod + bytemuck::Zeroable> {
    buffer: wgpu::Buffer,
    data: Vec<T>,
    usage: wgpu::BufferUsages,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable> CpuBuffer<T> {
    pub fn with_capacity(
        device: &wgpu::Device,
        capacity: usize,
        usage: wgpu::BufferUsages,
    ) -> Self {
        let usage = usage | wgpu::BufferUsages::COPY_DST;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<T>() * capacity) as _,
            usage,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            usage,
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn batch<'a>(
        &'a mut self,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
    ) -> Batch<'a, T> {
        Batch::new(device, queue, self)
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
    
    pub(crate) fn slice(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(..)
    }
    
    pub(crate) fn len(&self) -> u32 {
        self.data.len() as u32
    }

}

pub struct Batch<'a, T: bytemuck::Pod + bytemuck::Zeroable> {
    start: usize,
    queue: &'a wgpu::Queue,
    device: &'a wgpu::Device,
    buffer: &'a mut CpuBuffer<T>,
}

impl<'a, T: bytemuck::Pod + bytemuck::Zeroable> Batch<'a, T> {
    pub fn new(
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        buffer: &'a mut CpuBuffer<T>,
    ) -> Self {
        let start = buffer.data.len();
        Self {
            start,
            queue,
            device,
            buffer,
        }
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        self.buffer.data.push(value);
    }
}

impl<'a, T: bytemuck::Pod + bytemuck::Zeroable> Drop for Batch<'a, T> {
    fn drop(&mut self) {
        if self.buffer.data.len() == 0 {
            return;
        }

        if (self.buffer.data.len() * std::mem::size_of::<T>()) as u64 > self.buffer.buffer.size() {
            self.buffer.buffer = self.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&self.buffer.data),
                usage: self.buffer.usage,
            });
        } else if self.buffer.data.len() > 0 {
            self.queue.write_buffer(
                &self.buffer.buffer,
                (self.start * std::mem::size_of::<T>()) as _,
                bytemuck::cast_slice(&self.buffer.data[self.start..]),
            );
        }
    }
}
