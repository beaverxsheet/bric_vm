use std::fmt::Debug;

/// Custom Error class including all errors for the VM and associated tools
#[derive(Debug)]
pub enum BError {
    /// Instruction Parsing Error
    InstParseError { value: u16, message: String },
    /// Execution Halted Error
    ExecutionHaltedError { value: u16 },
    /// Invalid Instruction Error
    InvalidInstructionError { instruction: u16 },
    /// Region Map Error
    MapError(String),
    /// Out of Bounds Error
    OutOfBoundsError(u16, usize, usize),
    /// IO Error
    IoError(String),
    /// Assembly Parse Error
    AsmParseError(String),
    /// Serialization Error
    SerializationError(String),
    /// Deserialization Error
    DeserializationError(String),
}

impl std::fmt::Display for BError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BError::InstParseError { value, message } => {
                write!(f, "unable to parse {value}: {message}")
            }
            BError::ExecutionHaltedError { value } => {
                write!(f, "instruction {value} in ROM holds no data")
            }
            BError::InvalidInstructionError { instruction } => {
                write!(f, "instruction {} isn't valid", instruction)
            }
            BError::MapError(message) => {
                write!(f, "invalid regions: {}", message)
            }
            BError::OutOfBoundsError(address, size, max) => {
                write!(
                    f,
                    "the size of {size} at address {address} is to large for the underlying structure fo size {max}"
                )
            }
            BError::IoError(message) => {
                write!(f, "input / output error: {}", message)
            }
            BError::AsmParseError(message) => {
                write!(f, "unable to parse assembly: {message}")
            }
            BError::SerializationError(message) => {
                write!(f, "error serializing: {message}")
            }
            BError::DeserializationError(message) => {
                write!(f, "error deserializing: {message}")
            }
        }
    }
}

/// Represents a labeled interval between two numbers
#[derive(Debug, Clone)]
pub struct Region<K, V> {
    start: K,
    end: K,
    label: V,
}

impl<K, V> Region<K, V> {
    pub fn new(start: K, end: K, label: V) -> Self {
        Self { start, end, label }
    }
}

/// Represents a map of regions that are labeled that can be easily searched
#[derive(Debug, Clone)]
pub struct RegionMap<K, V> {
    regions: Vec<Region<K, V>>,
}

impl<K, V> TryFrom<Vec<Region<K, V>>> for RegionMap<K, V>
where
    K: Ord + Copy + Debug,
    V: Clone + Debug,
{
    type Error = BError;
    fn try_from(mut value: Vec<Region<K, V>>) -> Result<Self, Self::Error> {
        // verify start <= end
        for region in &value {
            if region.start > region.end {
                return Err(BError::MapError(format!(
                    "region {:?} has start > end",
                    region
                )));
            }
        }

        value.sort_by(|a, b| a.start.cmp(&b.start));

        // verify no overlaps
        for i in 1..value.len() {
            if value[i].start <= value[i - 1].end {
                return Err(BError::MapError(format!(
                    "regions {:?} and {:?} overlap",
                    value[i - 1],
                    value[i]
                )));
            }
        }

        Ok(Self { regions: value })
    }
}

impl<K, V> RegionMap<K, V>
where
    K: Ord + Copy + Debug,
    V: Clone + Debug,
{
    /// Uses bisection search to find the label associated with the position given.
    pub fn find_region(&self, position: K) -> Option<&V> {
        let mut low = 0;
        let mut high = self.regions.len();

        while low < high {
            let mid = (low + high) / 2;
            let region = &self.regions[mid];

            if position < region.start {
                high = mid;
            } else if position > region.end {
                low = mid + 1;
            } else {
                return Some(&region.label);
            }
        }

        None
    }
}

pub(crate) fn check_slice(input: &[u8], len: usize) -> Result<&[u8], BError> {
    if len > input.len() {
        Err(BError::SerializationError(
            "File to short or part missing".to_string(),
        ))
    } else {
        Ok(&input[..len])
    }
}

/// Extracts a number from a slice
/// panics if slice length < 3
/// errors if slice[3] != 0
pub(crate) fn extract_number(slice: &[u8]) -> Result<u16, BError> {
    let out = u16::from_be_bytes([slice[0], slice[1]]);
    if slice[2] != 0x00 {
        return Err(BError::SerializationError(
            "Invalid region separators".to_string(),
        ));
    }
    Ok(out)
}

/// Get a number from a string looking like `0xabc`, `0b01` or `10`
/// Error when conversion fails
pub fn number_literal_to_u16(input: &str) -> Result<u16, ()> {
    if input.starts_with("0x") {
        u16::from_str_radix(&input[2..], 16)
    } else if input.starts_with("0b") {
        u16::from_str_radix(&input[2..], 2)
    } else {
        u16::from_str_radix(input, 10)
    }
    .map_err(|_| ())
}
