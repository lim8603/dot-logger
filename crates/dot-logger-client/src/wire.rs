//! Internal checked-in protocol; both copies are checked against logger.proto.
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Source {
    #[prost(sint64, tag = "1")]
    pub ticks: i64,
    #[prost(uint64, tag = "2")]
    pub clock_domain: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Record {
    #[prost(bytes = "vec", tag = "1")]
    pub session: Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub producer: u32,
    #[prost(uint64, tag = "3")]
    pub sequence: u64,
    #[prost(uint32, tag = "4")]
    pub channel: u32,
    #[prost(bytes = "vec", tag = "5")]
    pub schema: Vec<u8>,
    #[prost(uint32, tag = "6")]
    pub schema_version: u32,
    #[prost(uint64, tag = "7")]
    pub revision: u64,
    #[prost(uint64, tag = "8")]
    pub mono: u64,
    #[prost(sint64, tag = "9")]
    pub real: i64,
    #[prost(message, optional, tag = "10")]
    pub source: Option<Source>,
    #[prost(uint32, tag = "11")]
    pub level: u32,
    #[prost(uint32, tag = "12")]
    pub importance: u32,
    #[prost(uint64, tag = "13")]
    pub payload_length: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct QueryMessage {
    #[prost(uint32, tag = "1")]
    pub basis: u32,
    #[prost(uint64, tag = "2")]
    pub clock_domain: u64,
    #[prost(bytes = "vec", tag = "3")]
    pub from: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub to: Vec<u8>,
    #[prost(uint32, optional, tag = "5")]
    pub channel: Option<u32>,
    #[prost(uint32, tag = "6")]
    pub minimum_level: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Request {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint64, tag = "2")]
    pub handle: u64,
    #[prost(uint64, tag = "3")]
    pub cursor: u64,
    #[prost(message, optional, tag = "4")]
    pub query: Option<QueryMessage>,
    #[prost(message, optional, tag = "5")]
    pub record: Option<Record>,
    #[prost(uint32, tag = "6")]
    pub target: u32,
    #[prost(uint32, tag = "7")]
    pub boundary: u32,
    #[prost(uint64, tag = "8")]
    pub offset: u64,
    #[prost(uint32, tag = "9")]
    pub length: u32,
    #[prost(uint64, tag = "10")]
    pub generation: u64,
    #[prost(uint32, tag = "11")]
    pub action: u32,
    #[prost(uint64, tag = "12")]
    pub from: u64,
    #[prost(uint32, tag = "13")]
    pub numerator: u32,
    #[prost(uint32, tag = "14")]
    pub denominator: u32,
    #[prost(bytes = "vec", tag = "15")]
    pub data: Vec<u8>,
    #[prost(uint64, tag = "16")]
    pub other: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Item {
    #[prost(message, optional, tag = "1")]
    pub record: Option<Record>,
    #[prost(bytes = "vec", tag = "2")]
    pub inline: Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub blob: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Reply {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint32, tag = "2")]
    pub kind: u32,
    #[prost(bytes = "vec", tag = "3")]
    pub session: Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub revision: u64,
    #[prost(uint64, tag = "5")]
    pub handle: u64,
    #[prost(uint64, tag = "6")]
    pub cursor: u64,
    #[prost(uint64, tag = "7")]
    pub generation: u64,
    #[prost(uint32, tag = "8")]
    pub state: u32,
    #[prost(message, repeated, tag = "9")]
    pub items: Vec<Item>,
    #[prost(bytes = "vec", tag = "10")]
    pub data: Vec<u8>,
    #[prost(uint32, tag = "11")]
    pub confirmed: u32,
    #[prost(uint32, tag = "12")]
    pub outcome: u32,
    #[prost(uint32, tag = "13")]
    pub reason: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct SubscribeRequest {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint32, optional, tag = "2")]
    pub channel: Option<u32>,
    #[prost(uint32, tag = "3")]
    pub minimum_level: u32,
    #[prost(bool, tag = "4")]
    pub originals: bool,
    #[prost(bool, tag = "5")]
    pub disconnect: bool,
    #[prost(uint32, tag = "6")]
    pub records: u32,
    #[prost(uint64, tag = "7")]
    pub bytes: u64,
    #[prost(uint64, tag = "8")]
    pub age_ms: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct Frame {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint64, tag = "2")]
    pub subscription: u64,
    #[prost(uint64, tag = "3")]
    pub transfer: u64,
    #[prost(uint64, tag = "4")]
    pub generation: u64,
    #[prost(message, optional, tag = "5")]
    pub record: Option<Record>,
    #[prost(uint64, tag = "6")]
    pub total: u64,
    #[prost(uint64, tag = "7")]
    pub offset: u64,
    #[prost(uint32, tag = "8")]
    pub checksum: u32,
    #[prost(bytes = "vec", tag = "9")]
    pub data: Vec<u8>,
    #[prost(bool, tag = "10")]
    pub original: bool,
    #[prost(bool, tag = "11")]
    pub late: bool,
    #[prost(uint64, tag = "12")]
    pub gap_first: u64,
    #[prost(uint64, tag = "13")]
    pub gap_last: u64,
    #[prost(uint32, tag = "14")]
    pub kind: u32,
    #[prost(uint32, tag = "15")]
    pub gap_reason: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct AckRequest {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint64, tag = "2")]
    pub subscription: u64,
    #[prost(uint64, tag = "3")]
    pub transfer: u64,
    #[prost(uint64, tag = "4")]
    pub generation: u64,
    #[prost(bytes = "vec", tag = "5")]
    pub session: Vec<u8>,
    #[prost(uint32, tag = "6")]
    pub producer: u32,
    #[prost(uint64, tag = "7")]
    pub sequence: u64,
}
