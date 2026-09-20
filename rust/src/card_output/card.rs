#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq)]
pub enum CardType {
    IMPORTANT,
    TIP,
    FEEDBACK,
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct Card {
    pub title: String,
    pub value: String,
    pub description: String,
    pub card_type: CardType,
}

#[uniffi::export]
pub fn test() -> Card {
    Card {
        title: "test".to_string(),
        value: "100%".to_string(),
        description: "test".to_string(),
        card_type: CardType::IMPORTANT,
    }
}