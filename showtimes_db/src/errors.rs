//! A collection of tiny errors handling

/// The result type
pub type SHDbResult<T, E = Error> = std::result::Result<T, E>;

/// The error collector
pub enum Error {
    /// Unknown error
    Unknown,
    /// A string validation error
    StringError(StringValidationError),
    /// Time conversion fails
    TimeConversionError(i64),
}

/// A structure to hold the validation error for a string
pub struct StringValidationError {
    /// The kind of error
    kind: StringValidationErrorKind,
    /// The error message
    key: String,
}

impl StringValidationError {
    pub(crate) fn new(key: impl Into<String>, kind: StringValidationErrorKind) -> Self {
        StringValidationError {
            key: key.into(),
            kind,
        }
    }

    /// The key of the error, usually the name of the field that caused the error
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The kind of error.
    pub fn kind(&self) -> &StringValidationErrorKind {
        &self.kind
    }
}

/// The error kind for streing validation
#[derive(Clone)]
pub enum StringValidationErrorKind {
    /// The string is empty
    Empty,
    /// The string is too long
    TooLong,
    /// The string is too short
    TooShort,
    /// Cannot contains specific characters
    Contains(String),
    /// Only contains ASCII
    ASCIIOnly,
}

impl std::fmt::Debug for StringValidationErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::fmt::Display for StringValidationErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringValidationErrorKind::Empty => write!(f, "cannot be empty"),
            StringValidationErrorKind::TooLong => write!(f, "is too long"),
            StringValidationErrorKind::TooShort => write!(f, "is too short"),
            StringValidationErrorKind::Contains(chars) => write!(f, "can only contains {}", chars),
            StringValidationErrorKind::ASCIIOnly => write!(f, "can only contains ASCII"),
        }
    }
}

impl std::fmt::Debug for StringValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.key)
    }
}

impl std::fmt::Display for StringValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}` {}", self.key, self.kind)
    }
}

impl From<StringValidationError> for Error {
    fn from(value: StringValidationError) -> Self {
        Error::StringError(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unknown => write!(f, "An unknown error has occurred"),
            Error::StringError(err) => write!(f, "{}", err),
            Error::TimeConversionError(number) => {
                write!(f, "Failed to convert timestamp: {}", number)
            }
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unknown => write!(f, "An unknown error has occurred"),
            Error::StringError(err) => write!(f, "{:?}", err),
            Error::TimeConversionError(number) => {
                write!(f, "Failed to convert timestamp: {:?}", number)
            }
        }
    }
}

impl std::error::Error for Error {}
impl std::error::Error for StringValidationError {}
