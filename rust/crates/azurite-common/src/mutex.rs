use std::{
    collections::{HashMap, VecDeque},
    sync::LazyLock,
};

use tokio::sync::{oneshot, Mutex as TokioMutex};

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MutexLockStatus {
    LOCKED,
    UNLOCKED,
}

static KEYS: LazyLock<TokioMutex<HashMap<String, MutexLockStatus>>> =
    LazyLock::new(|| TokioMutex::new(HashMap::new()));
static LISTENERS: LazyLock<TokioMutex<HashMap<String, VecDeque<oneshot::Sender<()>>>>> =
    LazyLock::new(|| TokioMutex::new(HashMap::new()));

#[derive(Debug, Default)]
pub struct Mutex;

impl Mutex {
    pub fn new() -> Self {
        Self
    }

    pub async fn lock(key: &str) {
        let receiver = {
            let mut keyMap = KEYS.lock().await;
            match keyMap.get(key) {
                Some(MutexLockStatus::LOCKED) => {
                    let (sender, receiver) = oneshot::channel();
                    drop(keyMap);
                    Self::onUnlockEvent(key, sender).await;
                    Some(receiver)
                }
                Some(MutexLockStatus::UNLOCKED) | None => {
                    keyMap.insert(key.to_string(), MutexLockStatus::LOCKED);
                    None
                }
            }
        };

        if let Some(receiver) = receiver {
            let _ = receiver.await;
        }
    }

    pub async fn unlock(key: &str) {
        let isLocked = {
            let keyMap = KEYS.lock().await;
            keyMap.get(key) == Some(&MutexLockStatus::LOCKED)
        };
        if !isLocked {
            return;
        }

        if Self::emitUnlockEvent(key).await {
            let mut keyMap = KEYS.lock().await;
            keyMap.insert(key.to_string(), MutexLockStatus::LOCKED);
        } else {
            let mut keyMap = KEYS.lock().await;
            keyMap.insert(key.to_string(), MutexLockStatus::UNLOCKED);
            keyMap.remove(key);
        }
    }

    async fn onUnlockEvent(key: &str, handler: oneshot::Sender<()>) {
        let mut listenerMap = LISTENERS.lock().await;
        listenerMap
            .entry(key.to_string())
            .or_insert_with(VecDeque::new)
            .push_back(handler);
    }

    async fn emitUnlockEvent(key: &str) -> bool {
        let mut listenerMap = LISTENERS.lock().await;
        let Some(queue) = listenerMap.get_mut(key) else {
            return false;
        };
        let Some(handler) = queue.pop_front() else {
            return false;
        };
        let _ = handler.send(());
        if queue.is_empty() {
            listenerMap.remove(key);
        }
        true
    }
}
