use std::fmt::Debug;
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::ThrottleError;

pub(crate) mod any;
pub(crate) mod map;
pub(crate) mod vec;

const CHANNEL_SIZE: usize = 100;
const WRONG_ARGS: &str = "Incorrect arguments have been provided for this method";
const WRONG_RESPONSE: &str = "An incorrect response type for this method has been called";

#[derive(Error, Debug, PartialEq, Clone)]
pub enum ActorError {
    #[error("A request has been received for type {0} while no value is set")]
    NoValueSet(String),
    #[error("Tokio oneshot receiver error")]
    TokioOneshotRecvError(#[from] oneshot::error::RecvError),
    #[error("Tokio mpsc sender error: {0}")]
    TokioMpscSendError(String),
    #[error("Tokio broadcast try receiver error")]
    TokioBroadcastTryRecvError(#[from] broadcast::error::TryRecvError),
    #[error("Tokio broadcast receiver error")]
    TokioBroadcastRecvError(#[from] broadcast::error::RecvError),
    #[error("An incorrect response type for this method has been received")]
    WrongResponse,
    #[error("A throttle error occured")]
    ThrottleError(#[from] ThrottleError),
}

impl<T> From<mpsc::error::SendError<T>> for ActorError {
    fn from(err: mpsc::error::SendError<T>) -> ActorError {
        ActorError::TokioMpscSendError(err.to_string())
    }
}

// TODO this should not be part of the actor model lib!
// Convert any actor error to an gRPC internal failure status
impl From<ActorError> for tonic::Status {
    fn from(e: ActorError) -> Self {
        tonic::Status::from_error(Box::new(e))
    }
}

#[cfg(test)]
mod tests {
    use crate::Handle;
    use crate::MapHandle;
    use crate::VecHandle;

    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn receive_val_broadcast() {
        let handle = Handle::new();
        let mut rx = handle.subscribe();
        handle.set(Some("testing!")).await.unwrap();
        assert_eq!(rx.recv().await.unwrap(), Some("testing!"));
    }

    #[tokio::test]
    async fn set_ok_actor() {
        let handle = Handle::new();
        handle.set(1).await.unwrap();
    }

    #[tokio::test]
    async fn get_ok_actor() {
        let handle = Handle::new();
        handle.set(1).await.unwrap();
        let result = handle.get().await;
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn get_err_actor() {
        let handle = Handle::<i32>::new();
        assert!(handle.get().await.is_err());
    }

    #[tokio::test]
    async fn no_val_set_actor() {
        let handle = Handle::<Vec<i32>>::new();
        let err = handle.push(10).await.unwrap_err();
        assert!(matches!(err, ActorError::NoValueSet(_)))
    }

    #[tokio::test]
    async fn push_to_actor() {
        let handle = Handle::new();
        handle.set(vec![1, 2]).await.unwrap();
        handle.push(100).await.unwrap();
        assert_eq!(handle.get().await.unwrap(), vec![1, 2, 100]);
    }

    #[tokio::test]
    async fn drain_actor() {
        let handle = Handle::new();
        handle.set(vec![1, 2]).await.unwrap();
        let res = handle.drain().await.unwrap();
        assert_eq!(res, vec![1, 2]);
        assert_eq!(handle.get().await.unwrap(), Vec::<i32>::new());
    }

    #[tokio::test]
    async fn insert_at_actor() {
        let handle = Handle::new();
        handle.set(HashMap::new()).await.unwrap();
        let res = handle.insert("test", 10).await.unwrap();
        assert_eq!(res, None);
    }

    #[tokio::test]
    async fn insert_overwrite_at_actor() {
        let handle = Handle::new();
        handle.set(HashMap::new()).await.unwrap();
        handle.insert("test", 10).await.unwrap();
        let old_value = handle.insert("test", 20).await.unwrap();
        assert_eq!(old_value, Some(10));
    }

    #[tokio::test]
    async fn get_key_actor() {
        let handle = Handle::new();
        handle.set(HashMap::new()).await.unwrap();
        handle.insert("test", 10).await.unwrap();
        let res = handle.get_key("test").await.unwrap();
        assert_eq!(res, Some(10));
    }

    #[tokio::test]
    async fn get_empty_from_actor() {
        let handle = Handle::new();
        handle.set(HashMap::<&str, i32>::new()).await.unwrap();
        let res = handle.get_key("test").await.unwrap();
        assert_eq!(res, None);
    }

    #[tokio::test]
    async fn actor_is_none() {
        let handle = Handle::<i32>::new();
        assert_eq!(handle.is_none().await.unwrap(), true);
    }

    #[tokio::test]
    async fn actor_is_not_none() {
        let handle = Handle::new();
        handle.set(1).await.unwrap();
        assert_eq!(handle.is_none().await.unwrap(), false);
    }

    #[tokio::test]
    async fn actor_hashmap_is_empty() {
        let handle = Handle::new_from(HashMap::<&str, i32>::new());
        assert_eq!(handle.is_empty().await.unwrap(), true);
    }

    #[tokio::test]
    async fn actor_hashmap_is_not_empty() {
        let handle = Handle::new_from(HashMap::new());
        handle.insert("test", 1).await.unwrap();
        assert_eq!(handle.is_none().await.unwrap(), false);
    }

    #[tokio::test]
    async fn actor_vec_is_empty() {
        let handle = Handle::new_from(Vec::<i32>::new());
        assert_eq!(handle.is_empty().await.unwrap(), true);
    }

    #[tokio::test]
    async fn actor_vec_is_not_empty() {
        let handle = Handle::new_from(Vec::new());
        handle.push(1).await.unwrap();
        assert_eq!(handle.is_none().await.unwrap(), false);
    }
}
