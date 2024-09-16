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

use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    Stream, StreamExt,
};
use slab::Slab;

type Brokers = HashMap<TypeId, Box<dyn Any + Send>>;

static BROKERS: LazyLock<Mutex<Brokers>> = LazyLock::new(|| Mutex::new(Brokers::new()));

struct Senders<T>(Slab<UnboundedSender<T>>);
struct BrokerStream<T: Sync + Send + Clone + 'static>(usize, UnboundedReceiver<T>);

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
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.1.poll_next_unpin(cx)
    }
}

/// A simple memory broker
pub struct MemoryBroker<T>(PhantomData<T>);

impl<T: Sync + Send + Clone + 'static> MemoryBroker<T> {
    /// Publish a message to the broker
    pub fn publish(msg: T) {
        with_senders::<T, _, _>(|senders| {
            for (_, sender) in senders.0.iter_mut() {
                sender.start_send(msg.clone()).ok();
            }
        })
    }

    /// Subscribe to the message of the specified type and returns a `Stream`.
    pub fn subscribe() -> impl Stream<Item = T> {
        with_senders::<T, _, _>(|senders| {
            let (tx, rx) = futures::channel::mpsc::unbounded();
            let id = senders.0.insert(tx);
            BrokerStream(id, rx)
        })
    }
}
