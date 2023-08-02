use serde::{de, ser};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum TryParse<T> {
    Parsed(T),
    Unparsed,
    #[default]
    NotPresent,
}

impl<T> ser::Serialize for TryParse<T>
where
    T: ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        Option::<&T>::from(self).serialize(serializer)
    }
}

impl<T> bincode::Encode for TryParse<T>
where
    T: bincode::Encode,
{
    fn encode<E>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError>
    where
        E: bincode::enc::Encoder,
    {
        Option::<&T>::from(self).encode(encoder)
    }
}

impl<'de, T> de::Deserialize<'de> for TryParse<T>
where
    T: de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        match Option::<T>::deserialize(deserializer) {
            Ok(Some(value)) => Ok(TryParse::Parsed(value)),
            Ok(None) => Ok(TryParse::NotPresent),
            Err(_) => Ok(TryParse::Unparsed),
        }
    }
}

impl<T> bincode::Decode for TryParse<T>
where
    T: bincode::Decode,
{
    fn decode<D>(decoder: &mut D) -> Result<Self, bincode::error::DecodeError>
    where
        D: bincode::de::Decoder,
    {
        match Option::<T>::decode(decoder) {
            Ok(Some(value)) => Ok(TryParse::Parsed(value)),
            Ok(None) => Ok(TryParse::NotPresent),
            Err(_) => Ok(TryParse::Unparsed),
        }
    }
}

impl<'de, T> bincode::BorrowDecode<'de> for TryParse<T>
where
    T: bincode::BorrowDecode<'de>,
{
    fn borrow_decode<D>(decoder: &mut D) -> Result<Self, bincode::error::DecodeError>
    where
        D: bincode::de::BorrowDecoder<'de>,
    {
        match Option::<T>::borrow_decode(decoder) {
            Ok(Some(value)) => Ok(TryParse::Parsed(value)),
            Ok(None) => Ok(TryParse::NotPresent),
            Err(_) => Ok(TryParse::Unparsed),
        }
    }
}

impl<T> From<TryParse<T>> for Option<T> {
    fn from(try_parse: TryParse<T>) -> Self {
        match try_parse {
            TryParse::Parsed(parsed) => Some(parsed),
            _ => None,
        }
    }
}

impl<'a, T> From<&'a TryParse<T>> for Option<&'a T> {
    fn from(try_parse: &'a TryParse<T>) -> Self {
        match try_parse {
            TryParse::Parsed(parsed) => Some(parsed),
            _ => None,
        }
    }
}
