//! The broker event system that use in-memory storage to store the events temporarily
//! before needing to be consumed by ClickHouse and the other services.
//!
//! Code is based on async-graphql's [broker example](https://github.com/async-graphql/examples/blob/master/models/books/src/simple_broker.rs).
//! Adapted for latest Rust.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    pin::Pin,
    sync::{LazyLock, Mutex},
    task::{Context, Poll},
};

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{Stream, StreamExt};
use slab::Slab;

use crate::models::SHEvent;

type Brokers = HashMap<TypeId, Box<dyn Any + Send>>;

static BROKERS: LazyLock<Mutex<Brokers>> = LazyLock::new(|| Mutex::new(Brokers::new()));

struct Senders<T: Sync + Send + Clone + 'static>(Slab<UnboundedSender<SHEvent<T>>>);
struct BrokerStream<T: Sync + Send + Clone + 'static>(usize, UnboundedReceiver<SHEvent<T>>);

fn with_senders<T, F, R>(f: F) -> R
where
    T: Send + Sync + Clone + 'static,
    F: FnOnce(&mut Senders<T>) -> R,
{
    let mut map = BROKERS.lock().unwrap();

    let senders = map
        .entry(TypeId::of::<Senders<T>>())
        .or_insert_with(|| Box::new(Senders::<T>(Default::default())));

    f(senders.downcast_mut::<Senders<T>>().unwrap())
}

impl<T: Sync + Send + Clone + 'static> Drop for BrokerStream<T> {
    fn drop(&mut self) {
        with_senders::<T, _, _>(|senders| senders.0.remove(self.0));
    }
}

impl<T: Sync + Send + Clone + 'static> Stream for BrokerStream<T> {
    type Item = SHEvent<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.1.poll_next_unpin(cx)
    }
}

/// A simple memory broker
pub struct MemoryBroker<T>(PhantomData<T>);

impl<T: Sync + Send + Clone + 'static> MemoryBroker<T> {
    /// Publish a message to the broker
    pub fn publish(msg: SHEvent<T>) {
        with_senders::<T, _, _>(|senders| {
            tracing::debug!(
                "Publishing message of type {:?} to {} subscribers",
                std::any::type_name::<T>(),
                senders.0.len()
            );
            for (_, sender) in senders.0.iter_mut() {
                tracing::trace!(
                    "Publishing message of type {:?} to {:?}",
                    std::any::type_name::<T>(),
                    sender
                );
                sender.start_send(msg.clone()).ok();
            }
        })
    }

    /// Subscribe to the message of the specified type and returns a `Stream`.
    pub fn subscribe() -> impl Stream<Item = SHEvent<T>> {
        with_senders::<T, _, _>(|senders| {
            let (tx, rx) = futures_channel::mpsc::unbounded();
            let id = senders.0.insert(tx);
            tracing::trace!(
                "Subscribing for message type {:?} with ID {}",
                std::any::type_name::<T>(),
                id
            );
            BrokerStream(id, rx)
        })
    }
}
