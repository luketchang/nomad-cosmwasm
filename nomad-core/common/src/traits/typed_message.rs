pub trait TypedMessage {
    type MessageEnum;

    /// Return the message type
    fn message_type(&self) -> Self::MessageEnum;
}