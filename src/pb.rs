// Auto-generated protobuf code for trill.signaling
// Generated from signaling.proto

// use prost::Message; // unused

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Packet {
    #[prost(oneof = "packet::Which", tags = "4, 1, 2, 3, 5, 6, 7")]
    pub which: ::core::option::Option<packet::Which>,
}

pub mod packet {
    // use prost::Message; // unused

    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Which {
        #[prost(message, tag = "4")]
        Hello(super::Hello),
        #[prost(message, tag = "1")]
        Start(super::Start),
        #[prost(message, tag = "2")]
        Offer(super::Offer),
        #[prost(message, tag = "3")]
        Answer(super::Answer),
        #[prost(message, tag = "5")]
        Abort(super::Abort),
        #[prost(message, tag = "6")]
        Ping(super::Ping),
        #[prost(message, tag = "7")]
        IceCandidate(super::IceCandidate),
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Hello {
        #[prost(message, repeated, tag = "1")]
        pub ice_servers: ::prost::alloc::vec::Vec<hello::IceServer>,
    }

    pub mod hello {
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct IceServer {
            #[prost(string, repeated, tag = "3")]
            pub urls: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
            #[prost(string, optional, tag = "2")]
            pub username: ::core::option::Option<::prost::alloc::string::String>,
            #[prost(string, optional, tag = "1")]
            pub credential: ::core::option::Option<::prost::alloc::string::String>,
        }
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Start {
        #[prost(uint32, tag = "1")]
        pub protocol_version: u32,
        #[prost(string, tag = "2")]
        pub offer_sdp: ::prost::alloc::string::String,
        #[prost(bytes, tag = "3")]
        pub connection_id: ::prost::bytes::Bytes,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Offer {
        #[prost(string, tag = "1")]
        pub sdp: ::prost::alloc::string::String,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Answer {
        #[prost(string, tag = "1")]
        pub sdp: ::prost::alloc::string::String,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Abort {
        #[prost(enumeration = "abort::Reason", tag = "1")]
        pub reason: i32,
    }

    pub mod abort {
        // use prost::Enumeration; // unused

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(i32)]
        pub enum Reason {
            Unknown = 0,
            ProtocolVersionTooOld = 1,
            ProtocolVersionTooNew = 2,
            MissingSessionId = 3,
            NotUpgrade = 4,
        }

        impl Reason {
            /// String value of the enum field names used in the ProtoBuf definition.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    Reason::Unknown => "REASON_UNKNOWN",
                    Reason::ProtocolVersionTooOld => "REASON_PROTOCOL_VERSION_TOO_OLD",
                    Reason::ProtocolVersionTooNew => "REASON_PROTOCOL_VERSION_TOO_NEW",
                    Reason::MissingSessionId => "REASON_MISSING_SESSION_ID",
                    Reason::NotUpgrade => "REASON_NOT_UPGRADE",
                }
            }

            /// Converts an i32 to a possible instance of Reason.
            pub fn from_i32(value: i32) -> ::core::option::Option<Self> {
                match value {
                    0 => Some(Self::Unknown),
                    1 => Some(Self::ProtocolVersionTooOld),
                    2 => Some(Self::ProtocolVersionTooNew),
                    3 => Some(Self::MissingSessionId),
                    4 => Some(Self::NotUpgrade),
                    _ => None,
                }
            }
        }

        impl Default for Reason {
            fn default() -> Self {
                Self::Unknown
            }
        }

        impl ::core::convert::From<i32> for Reason {
            fn from(v: i32) -> Self {
                Self::from_i32(v).unwrap_or(Self::Unknown)
            }
        }
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Ping {}

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IceCandidate {
        #[prost(string, tag = "1")]
        pub candidate: ::prost::alloc::string::String,
    }
}

impl Packet {
    // Currently unused — the client-facing validation goes through
    // `is_server_only`. Kept for reference/future use.
    // pub fn is_valid_from_client(&self) -> bool {
    //     matches!(
    //         self.which.as_ref(),
    //         Some(packet::Which::Start(_))
    //             | Some(packet::Which::Answer(_))
    //             | Some(packet::Which::Ping(_))
    //             | Some(packet::Which::IceCandidate(_))
    //     )
    // }

    pub fn is_server_only(&self) -> bool {
        matches!(
            self.which.as_ref(),
            Some(packet::Which::Hello(_))
                | Some(packet::Which::Offer(_))
                | Some(packet::Which::Abort(_))
        )
    }
}

pub use packet::{Abort, Answer, Hello, IceCandidate, Offer, Ping, Start};
