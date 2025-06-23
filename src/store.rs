use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub trait StoreData: Default {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(bytes: &[u8]) -> std::io::Result<Self>
    where
        Self: Sized;
}

pub struct Store<T: StoreData> {
    data: Mutex<T>,
    store_path: PathBuf,
}

impl<T: StoreData> Store<T> {
    pub fn new(store_path: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(store_path.parent().unwrap_or(std::path::Path::new("")))?;

        let data: T = if !store_path.exists() {
            log::warn!(
                "store file {} does not exist - creating new",
                store_path.display()
            );
            Default::default()
        } else {
            log::trace!("reading from existing store file {}", store_path.display());
            fs::read(&store_path).and_then(|val| StoreData::deserialize(&val))?
        };

        Ok(Self {
            data: Mutex::new(data),
            store_path,
        })
    }
    pub fn with_mut<F: FnOnce(&mut T)>(&mut self, fun: F) {
        let val = &mut self.data.lock().unwrap();
        fun(val);
        self.flush_not_thread_safe(val);
    }
    pub fn with<F: FnOnce(&T)>(&self, fun: F) {
        let val = &self.data.lock().unwrap();
        fun(val);
    }
    fn flush_not_thread_safe(&self, val: &T) {
        log::trace!("writing to store file {}", self.store_path.display());
        if let Some(err) = fs::write(&self.store_path, val.serialize().as_slice()).err() {
            log::error!(
                "error writing to store file {}: {}",
                self.store_path.display(),
                err
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temp;
    use crate::temp::FileDetails;
    
    #[derive(Default, Eq, PartialEq, Debug)]
    struct A {
        b: u8,
        c: u8,
    }
    
    impl StoreData for A {
        fn serialize(&self) -> Vec<u8> {
            vec![self.b, self.c]
        }

        fn deserialize(bytes: &[u8]) -> std::io::Result<Self>
        where
            Self: Sized
        {
            let s = match bytes.len() {
                1 => Self {b: bytes[0], c: 0},
                2 => Self {b: bytes[0], c: bytes[1]},
                _ => Self::default()
            };
            Ok(s)
        }
    }

    #[test]
    fn test_write_read() {
        let test_dir = temp::TempDir::new().unwrap();
        let parent = test_dir.get_path();
        let file = parent.join("test.txt");
        let mut store: Store<A> = Store::new(file.clone()).unwrap();
        store.with_mut(|val| {
            assert_eq!(val, &A::default());
            val.b = 10;
            val.c = 211;
        });
        
        drop(store);
        
        let store: Store<A> = Store::new(file).unwrap();
        store.with(|val| {
            assert_eq!(10, val.b);
            assert_eq!(211, val.c);
        })
    }
}
