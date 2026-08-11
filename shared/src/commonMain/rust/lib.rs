uniffi::setup_scaffolding!();

#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq)]
pub enum CardType {
   IMPORTANT,
   TIP,
   FEEDBACK
}


#[derive(uniffi::Record)]
pub struct Card {
   pub title: String,
   pub value: String,
   pub description: String,
   pub card_type: CardType
}

#[uniffi::export]
pub fn test() -> Card{
   let card = Card {
      title: "test".to_string(),
      value: "100%".to_string(),
      description: "test".to_string(),
      card_type: CardType::IMPORTANT,
   };
   card
}