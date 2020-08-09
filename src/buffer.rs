



struct SerialBuffer {
    buf: [u8; 128],
    index: u16,
    locked: bool
}

impl SerialBuffer {
    pub fn push(&self){

    }

    pub fn lock(&self){

    }

    pub fn unlock(&self){

    }

    pub fn is_locked(&self) -> bool {
        false
    }
}



