/// Fem input/output data properties
#[cfg_attr(features = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct Properties {
    #[cfg_attr(features = "serde", serde(rename = "nodeID"))]
    pub node_id: Option<Vec<u32>>,
    #[cfg_attr(features = "serde", serde(rename = "csLabel"))]
    pub cs_label: Option<String>,
    #[cfg_attr(features = "serde", serde(rename = "csNumber"))]
    pub cs_number: Option<Vec<u32>>,
    pub coefficients: Option<Vec<f64>>,
    pub location: Option<Vec<f64>>,
    pub component: Option<Vec<i32>>,
    pub components: Option<Vec<f64>>,
    pub area: Option<Vec<f64>>,
}
/// Fem input/output data
#[cfg_attr(features = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct IOData {
    #[allow(dead_code)]
    pub types: String,
    #[cfg_attr(features = "serde", serde(rename = "exciteIDs"))]
    #[allow(dead_code)]
    pub excite_ids: Option<Vec<u32>>,
    pub descriptions: String,
    pub indices: Vec<u32>,
    pub properties: Properties,
}
/// Fem input/output 2 states: on or off
#[cfg_attr(features = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(features = "serde", serde(untagged))]
pub enum IO {
    On(IOData),
    Off(IOData),
}
impl From<IO> for IOData {
    fn from(value: IO) -> Self {
        match value {
            IO::On(data) => data,
            IO::Off(data) => data,
        }
    }
}
impl IO {
    pub fn switch_off(self) -> Self {
        match self {
            IO::On(data) => IO::Off(data),
            IO::Off(_) => self,
        }
    }
    pub fn switch_on(self) -> Self {
        match self {
            IO::Off(data) => IO::On(data),
            IO::On(_) => self,
        }
    }
    pub fn switch_on_by<F>(self, pred: F) -> Self
    where
        F: Fn(&IOData) -> bool,
    {
        match self {
            IO::Off(data) if pred(&data) => IO::On(data),
            IO::Off(_) => self,
            IO::On(_) => self,
        }
    }
    pub fn get_by<F, T>(&self, pred: F) -> Option<T>
    where
        F: Fn(&IOData) -> Option<T>,
    {
        match self {
            IO::On(data) => pred(data),
            IO::Off(_) => None,
        }
    }
    pub fn is_on(&self) -> bool {
        match self {
            IO::On(_) => true,
            IO::Off(_) => false,
        }
    }
}
