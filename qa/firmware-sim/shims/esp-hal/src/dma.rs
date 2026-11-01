use core::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
pub struct DmaDescriptor;

#[derive(Clone, Copy, Debug)]
pub struct DmaError;

/// A receive buffer owned by value, as the firmware passes it through capture.
pub struct DmaRxBuf {
    storage: &'static mut [u8],
    received: usize,
}

impl DmaRxBuf {
    pub fn new(_descriptors: &'static mut [DmaDescriptor], storage: &'static mut [u8]) -> Result<Self, DmaError> {
        let received = storage.len();
        Ok(Self { storage, received })
    }

    pub fn as_slice(&self) -> &[u8] {
        &*self.storage
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut *self.storage
    }

    pub fn number_of_received_bytes(&self) -> usize {
        self.received
    }

    pub fn read_received_data(&self, output: &mut [u8]) -> usize {
        let count = output.len().min(self.received);
        output[..count].copy_from_slice(&self.storage[..count]);
        count
    }
}

pub struct DmaTransferTxCircular<'a, T> {
    pub(crate) _transmitter: PhantomData<&'a mut T>,
}

impl<T> DmaTransferTxCircular<'_, T> {
    /// The virtual codec drains instantly, so every chunk is accepted whole.
    pub fn push_with(&mut self, fill: impl FnOnce(&mut [u8]) -> usize) -> Result<usize, DmaError> {
        let mut chunk = [0u8; 4092];
        Ok(fill(&mut chunk))
    }

    pub fn stop(self) -> Result<(), DmaError> {
        Ok(())
    }
}

#[macro_export]
macro_rules! dma_buffers {
    ($rx:expr, $tx:expr) => {{
        let rx = Box::leak(Box::new([0u8; $rx]));
        let tx = Box::leak(Box::new([0u8; $tx]));
        let rx_descriptors: &'static mut [$crate::dma::DmaDescriptor] =
            Box::leak(vec![$crate::dma::DmaDescriptor; 8].into_boxed_slice());
        let tx_descriptors: &'static mut [$crate::dma::DmaDescriptor] =
            Box::leak(vec![$crate::dma::DmaDescriptor; 8].into_boxed_slice());
        (rx, rx_descriptors, tx, tx_descriptors)
    }};
}
