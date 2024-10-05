
use std::pin::Pin;
use futures::Stream;
use bytes::Bytes;
use reqwest::Error as ReqwestError;

pub type AccumulatedStream = Pin<Box<dyn Stream<Item = Result<Bytes, ReqwestError>> + Send>>;
