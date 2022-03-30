// Copyright (c) Facebook, Inc. and its affiliates.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;

use lazy_static::lazy_static;
use maplit::btreemap;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PayloadValue {
    id: Option<i64>,
    list: Option<Vec<String>>,
}

type Payload = BTreeMap<i32, PayloadValue>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct NoPayloadReordered {
    name: String,
    t: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WithPayload {
    t: u64,
    name: String,
    payload: Payload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WithOptionalPayload {
    t: u64,
    name: String,
    payload: Option<Payload>,
}

lazy_static! {
    static ref NO_PAYLOAD_REORDERED: NoPayloadReordered = NoPayloadReordered {
        name: "alice".to_owned(),
        t: 1234,
    };
    static ref WITH_PAYLOAD: WithPayload = WithPayload {
        name: "alice".to_owned(),
        t: 1234,
        payload: btreemap! {
            1 => PayloadValue {
                id: Some(1),
                list: Some(vec!["a".to_owned()]),
            },
            2 => PayloadValue {
                id: Some(1),
                list: Some(vec!["b".to_owned(), "cd".to_owned()]),
            }
        },
    };
    static ref WITH_NO_PAYLOAD: WithOptionalPayload = WithOptionalPayload {
        name: "alice".to_owned(),
        t: 1234,
        payload: None,
    };
    static ref WITH_SOME_PAYLOAD: WithOptionalPayload = WithOptionalPayload {
        name: "alice".to_owned(),
        t: 1234,
        payload: Some(btreemap! {
            1 => PayloadValue {
                id: Some(1),
                list: Some(vec!["a".to_owned()]),
            },
            2 => PayloadValue {
                id: Some(1),
                list: Some(vec!["b".to_owned(), "cd".to_owned()]),
            }
        }),
    };
}

#[test]
fn test_serialize_deserialize() {
    {
        let bytes = serde_cbor::to_vec(&*NO_PAYLOAD_REORDERED).unwrap();
        let res: NoPayloadReordered = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(res, *NO_PAYLOAD_REORDERED);
    }
    {
        let bytes = serde_cbor::to_vec(&*WITH_PAYLOAD).unwrap();
        let res: WithPayload = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(res, *WITH_PAYLOAD);
    }
    {
        let bytes = serde_cbor::to_vec(&*WITH_NO_PAYLOAD).unwrap();
        let res: WithOptionalPayload = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(res, *WITH_NO_PAYLOAD);
    }
    {
        let bytes = serde_cbor::to_vec(&*WITH_SOME_PAYLOAD).unwrap();
        let res: WithOptionalPayload = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(res, *WITH_SOME_PAYLOAD);
    }
}

#[test]
fn test_removing_field() {
    let bytes = serde_cbor::to_vec(&*WITH_PAYLOAD).unwrap();
    // Unknown `Payload` field.
    let res: NoPayloadReordered = serde_cbor::from_slice(&bytes).unwrap();
    assert_eq!(res, *NO_PAYLOAD_REORDERED);
}

#[test]
fn test_removing_optional_field() {
    let bytes = serde_cbor::to_vec(&*WITH_SOME_PAYLOAD).unwrap();
    // Unknown `Option<Payload>` field.
    let res: NoPayloadReordered = serde_cbor::from_slice(&bytes).unwrap();
    assert_eq!(res, *NO_PAYLOAD_REORDERED);
}

#[test]
fn test_adding_field() {
    let bytes = serde_cbor::to_vec(&*NO_PAYLOAD_REORDERED).unwrap();
    // Missing `Payload`. This should fail.
    serde_cbor::from_slice::<WithPayload>(&bytes)
        .expect_err("Should have failed deserializing due to missing required field");
}

#[test]
fn test_adding_optional_field() {
    let bytes = serde_cbor::to_vec(&*NO_PAYLOAD_REORDERED).unwrap();
    // Missing `Option<Payload>`. This deserialize as `None`.
    let res: WithOptionalPayload = serde_cbor::from_slice(&bytes).unwrap();
    assert_eq!(res, *WITH_NO_PAYLOAD);
}

#[test]
fn test_changing_required_to_optional() {
    let bytes = serde_cbor::to_vec(&*WITH_PAYLOAD).unwrap();
    // Deserializing `Payload` as `Option<Payload>`.
    let res: WithOptionalPayload = serde_cbor::from_slice(&bytes).unwrap();
    assert_eq!(res, *WITH_SOME_PAYLOAD);
}

#[test]
fn test_changing_optional_to_required() {
    let bytes = serde_cbor::to_vec(&*WITH_NO_PAYLOAD).unwrap();
    // Deserializing `Option<Payload>` as `Payload`. This should fail.
    serde_cbor::from_slice::<WithPayload>(&bytes)
        .expect_err("Should have failed deserializing as Option<T> cannot be deserialized to T");
}
