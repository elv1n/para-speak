use ringbuf::traits::{Consumer as _, Producer as _};
use ringbuf::{traits::*, HeapRb};
use std::sync::Mutex;

type Producer = ringbuf::HeapProd<u8>;
type Consumer = ringbuf::HeapCons<u8>;

pub struct RingBuffer {
    producer: Mutex<Producer>,
    consumer: Mutex<Consumer>,
}

impl RingBuffer {
    pub fn new(capacity_seconds: usize, sample_rate: u32) -> Self {
        let capacity = capacity_seconds * sample_rate as usize * 2;
        let ring = HeapRb::<u8>::new(capacity);
        let (producer, consumer) = ring.split();

        Self {
            producer: Mutex::new(producer),
            consumer: Mutex::new(consumer),
        }
    }

    pub fn write(&self, data: &[u8]) {
        if let Ok(mut producer) = self.producer.lock() {
            let written = producer.push_slice(data);
            if written < data.len() {
                log::warn!(
                    "[RingBuffer] Overflow: tried to write {} bytes, only wrote {}. Buffer full.",
                    data.len(),
                    written
                );
            }
        }
    }

    pub fn read_chunk(&self, min_size: usize) -> Option<Vec<u8>> {
        if let Ok(mut consumer) = self.consumer.lock() {
            let available = consumer.occupied_len();

            if available < min_size {
                return None;
            }

            let mut chunk = vec![0u8; available];
            let read = consumer.pop_slice(&mut chunk);
            chunk.truncate(read);

            Some(chunk)
        } else {
            None
        }
    }

    pub fn available_bytes(&self) -> usize {
        if let Ok(consumer) = self.consumer.lock() {
            consumer.occupied_len()
        } else {
            0
        }
    }

    pub fn reset_reader(&self) {
        if let Ok(mut consumer) = self.consumer.lock() {
            let len = consumer.occupied_len();
            if len > 0 {
                let mut dummy = vec![0u8; len];
                let _ = consumer.pop_slice(&mut dummy);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_write_read() {
        let buffer = Arc::new(RingBuffer::new(1, 48000));

        let data = vec![1, 2, 3, 4, 5];
        buffer.write(&data);

        let read = buffer.read_chunk(3).unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn test_min_size_threshold() {
        let buffer = Arc::new(RingBuffer::new(1, 48000));

        buffer.write(&[1, 2, 3]);
        assert!(buffer.read_chunk(10).is_none());
        assert!(buffer.read_chunk(2).is_some());
    }

    #[test]
    fn test_multiple_writes() {
        let buffer = Arc::new(RingBuffer::new(1, 48000));

        buffer.write(&[1, 2, 3]);
        buffer.write(&[4, 5, 6]);

        let read = buffer.read_chunk(1).unwrap();
        assert_eq!(read, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_reset_reader() {
        let buffer = Arc::new(RingBuffer::new(1, 48000));

        buffer.write(&[1, 2, 3, 4, 5]);
        assert_eq!(buffer.available_bytes(), 5);

        buffer.reset_reader();
        assert_eq!(buffer.available_bytes(), 0);
    }

    #[test]
    fn test_concurrent_write_read() {
        let buffer = Arc::new(RingBuffer::new(1, 48000));
        let buffer_write = Arc::clone(&buffer);
        let buffer_read = Arc::clone(&buffer);

        let writer = thread::spawn(move || {
            for i in 0..100 {
                let data = vec![i; 100];
                buffer_write.write(&data);
                thread::sleep(Duration::from_millis(1));
            }
        });

        let reader = thread::spawn(move || {
            let mut total_read = 0;
            for _ in 0..100 {
                if let Some(chunk) = buffer_read.read_chunk(50) {
                    total_read += chunk.len();
                }
                thread::sleep(Duration::from_millis(1));
            }
            total_read
        });

        writer.join().unwrap();
        let bytes_read = reader.join().unwrap();

        assert!(bytes_read > 0);
    }

    #[test]
    fn test_overflow_handling() {
        let buffer = Arc::new(RingBuffer::new(1, 48000));

        let huge_data = vec![0u8; 200_000];
        buffer.write(&huge_data);

        assert!(buffer.available_bytes() <= 96_000);
    }

    #[test]
    fn test_wrap_around() {
        let buffer = Arc::new(RingBuffer::new(1, 1000));

        for _ in 0..10 {
            buffer.write(&[1, 2, 3, 4, 5]);
            let _ = buffer.read_chunk(1);
        }

        buffer.write(&[42; 100]);
        let read = buffer.read_chunk(1).unwrap();
        assert!(read.contains(&42));
    }

    #[test]
    fn test_empty_buffer() {
        let buffer = Arc::new(RingBuffer::new(1, 48000));

        assert_eq!(buffer.available_bytes(), 0);
        assert!(buffer.read_chunk(1).is_none());
    }
}
