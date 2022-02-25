pub struct TypedView(Vec<u8>);

impl TypedView {
    pub fn new(view: Vec<u8>) -> Self {
        Self(view)
    }
}

impl AsRef<[u8]> for TypedView {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl std::ops::Deref for TypedView {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// First byte of Vec<u8> is a u8 corresponding to message type. This trait
/// describes provides structure for encoding Vec<u8> as xapp messages.
pub trait TypedMessage: AsRef<[u8]> {
    type MessageEnum: From<u8>;

    /// Return the message type
    fn message_type(&self) -> Self::MessageEnum {
        let slice: &[u8] = self.as_ref();
        slice[0].into()
    }
}
