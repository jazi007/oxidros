#[repr(C)]
#[derive(Debug)]
pub struct SequenceRaw<T> {
    pub data: *mut T,
    pub size: usize,
    pub capacity: usize,
}
impl<T> SequenceRaw<T> {
    pub const fn null() -> Self {
        unsafe { std::mem::MaybeUninit::zeroed().assume_init() }
    }
    pub fn as_slice(&self) -> &[T] {
        if self.data.is_null() {
            &[]
        } else {
            let s = unsafe { std::slice::from_raw_parts(self.data, self.size) };
            s
        }
    }
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        if self.data.is_null() {
            &mut []
        } else {
            let s = unsafe { std::slice::from_raw_parts_mut(self.data, self.size) };
            s
        }
    }
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_slice().iter()
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.as_slice_mut().iter_mut()
    }
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
