use crossbeam_channel::Sender;
use futuresdr::runtime::dev::prelude::*;

#[derive(Block)]
pub struct ChunkSink<I: CpuBufferReader<Item = f32> = DefaultCpuReader<f32>> {
    #[input]
    input: I,
    chunk_size: usize,
    buffer: Vec<f32>,
    tx: Sender<Vec<f32>>,
}

impl<I: CpuBufferReader<Item = f32>> ChunkSink<I> {
    pub fn new(chunk_size: usize, tx: Sender<Vec<f32>>) -> Self {
        Self {
            input: I::default(),
            chunk_size,
            buffer: Vec::with_capacity(chunk_size),
            tx,
        }
    }
}

#[doc(hidden)]
impl<I: CpuBufferReader<Item = f32>> Kernel for ChunkSink<I> {
    async fn work(
        &mut self,
        io: &mut WorkIo,
        _mo: &mut MessageOutputs,
        _meta: &mut BlockMeta,
    ) -> Result<()> {
        let samples: Vec<f32> = self.input().slice().to_vec();
        let n = samples.len();
        if n > 0 {
            self.input().consume(n);
            self.buffer.extend_from_slice(&samples);
            while self.buffer.len() >= self.chunk_size {
                let frame: Vec<f32> = self.buffer.drain(..self.chunk_size).collect();
                let _ = self.tx.try_send(frame);
            }
        }
        if self.input().finished() {
            io.finished = true;
        }
        Ok(())
    }
}
