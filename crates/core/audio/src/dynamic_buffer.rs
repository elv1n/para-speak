use config::Config;

pub struct DynamicBuffer {
    buffer: Vec<u8>,
    initial_capacity: usize,
    growth_increment: usize,
    last_logged_capacity: usize,
}

impl DynamicBuffer {
    pub fn new() -> Self {
        let config = Config::global();
        let initial_capacity = config.sample_rate as usize * 2 * config.initial_buffer_seconds as usize;
        let growth_increment = config.sample_rate as usize * 2 * 15;
        
        Self {
            buffer: Vec::with_capacity(initial_capacity),
            initial_capacity,
            growth_increment,
            last_logged_capacity: initial_capacity,
        }
    }
    
    #[cfg(test)]
    pub fn with_capacity(initial_capacity: usize) -> Self {
        let growth_increment = 48000 * 2 * 15;
        
        Self {
            buffer: Vec::with_capacity(initial_capacity),
            initial_capacity,
            growth_increment,
            last_logged_capacity: initial_capacity,
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        let required_capacity = self.buffer.len() + data.len();

        if required_capacity > self.buffer.capacity() {
            let new_capacity =
                ((required_capacity / self.growth_increment) + 1) * self.growth_increment;
            self.buffer.reserve(new_capacity - self.buffer.capacity());

            if new_capacity >= self.last_logged_capacity + self.growth_increment {
                log::debug!(
                    "DynamicBuffer capacity increased: {:.2} MB -> {:.2} MB (used: {:.2} MB)",
                    self.last_logged_capacity as f64 / 1_048_576.0,
                    new_capacity as f64 / 1_048_576.0,
                    required_capacity as f64 / 1_048_576.0
                );
                self.last_logged_capacity = new_capacity;
            }
        }

        self.buffer.extend_from_slice(data);
    }

    pub fn read_all(&mut self) -> Vec<u8> {
        let data = std::mem::take(&mut self.buffer);
        log::debug!(
            "Reading all data from DynamicBuffer: {} bytes ({:.2} MB)",
            data.len(),
            data.len() as f64 / 1_048_576.0
        );
        self.buffer = Vec::with_capacity(self.initial_capacity);
        data
    }

    pub fn reset(&mut self) {
        let previous_len = self.buffer.len();
        let previous_capacity = self.buffer.capacity();
        
        self.buffer.clear();
        self.buffer.shrink_to(self.initial_capacity);
        self.last_logged_capacity = self.initial_capacity;
        
        if previous_capacity > self.initial_capacity {
            log::debug!(
                "DynamicBuffer reset: {} bytes cleared, capacity shrunk from {} to {} bytes",
                previous_len,
                previous_capacity,
                self.buffer.capacity()
            );
        } else {
            log::debug!("DynamicBuffer reset: {} bytes cleared", previous_len);
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    #[allow(dead_code)]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    #[allow(dead_code)]
    pub fn get_data(&self) -> &[u8] {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_buffer_growth() {
        let mut buffer = DynamicBuffer::with_capacity(1024);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.capacity() >= 1024);

        let data = vec![0u8; 512];
        buffer.write(&data);
        assert_eq!(buffer.len(), 512);

        buffer.write(&data);
        assert_eq!(buffer.len(), 1024);

        buffer.write(&data);
        assert_eq!(buffer.len(), 1536);
        assert!(buffer.capacity() >= 1536);
    }

    #[test]
    fn test_read_all() {
        let mut buffer = DynamicBuffer::with_capacity(1024);
        let data = vec![1, 2, 3, 4, 5];
        buffer.write(&data);

        let read_data = buffer.read_all();
        assert_eq!(read_data, data);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_reset() {
        let mut buffer = DynamicBuffer::with_capacity(1024);
        let data = vec![1u8; 2048];
        buffer.write(&data);
        assert_eq!(buffer.len(), 2048);
        assert!(buffer.capacity() >= 2048);

        buffer.reset();
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.capacity(), 1024);
    }
}
