// Copyright (c) 2004-present, Facebook, Inc.
// All Rights Reserved.
//
// This software may be used and distributed according to the terms of the
// GNU General Public License version 2 or any later version.

use blobstore::Blobstore;
use blobstore_sync_queue::{BlobstoreSyncQueue, BlobstoreSyncQueueEntry};
use chrono::Duration as ChronoDuration;
use cloned::cloned;
use context::CoreContext;
use failure_ext::{err_msg, prelude::*};
use futures::{self, future::join_all, prelude::*};
use futures_ext::FutureExt;
use itertools::{Either, Itertools};
use lazy_static::lazy_static;
use metaconfig_types::BlobstoreId;
use mononoke_types::{BlobstoreBytes, DateTime};
use slog::{info, warn, Logger};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

lazy_static! {
    /// Minimal age of entry to consider if it has to be healed
    static ref ENTRY_HEALING_MIN_AGE: ChronoDuration = ChronoDuration::minutes(2);
}

pub struct Healer {
    logger: Logger,
    blobstore_sync_queue_limit: usize,
    sync_queue: Arc<dyn BlobstoreSyncQueue>,
    blobstores: Arc<HashMap<BlobstoreId, Arc<dyn Blobstore>>>,
    blobstore_key_like: Option<String>,
}

impl Healer {
    pub fn new(
        logger: Logger,
        blobstore_sync_queue_limit: usize,
        sync_queue: Arc<dyn BlobstoreSyncQueue>,
        blobstores: Arc<HashMap<BlobstoreId, Arc<dyn Blobstore>>>,
        blobstore_key_like: Option<String>,
    ) -> Self {
        Self {
            logger,
            blobstore_sync_queue_limit,
            sync_queue,
            blobstores,
            blobstore_key_like,
        }
    }

    /// Heal one batch of entries. It selects a set of entries which are not too young (bounded
    /// by ENTRY_HEALING_MIN_AGE) up to `blobstore_sync_queue_limit` at once.
    pub fn heal(&self, ctx: CoreContext) -> impl Future<Item = (), Error = Error> {
        cloned!(
            self.logger,
            self.blobstore_sync_queue_limit,
            self.sync_queue,
            self.blobstores,
        );

        let now = DateTime::now().into_chrono();
        let healing_deadline = DateTime::new(now - *ENTRY_HEALING_MIN_AGE);

        sync_queue
            .iter(
                ctx.clone(),
                self.blobstore_key_like.clone(),
                healing_deadline.clone(),
                blobstore_sync_queue_limit,
            )
            .and_then(move |queue_entries: Vec<BlobstoreSyncQueueEntry>| {
                let healing_futures: Vec<_> = queue_entries
                    .into_iter()
                    .group_by(|entry| entry.blobstore_key.clone())
                    .into_iter()
                    .filter_map(|(key, entries)| {
                        cloned!(ctx, sync_queue, blobstores, healing_deadline);
                        heal_blob(
                            ctx,
                            sync_queue,
                            blobstores,
                            healing_deadline,
                            key,
                            entries.collect(),
                        )
                    })
                    .collect();

                info!(
                    logger,
                    "Found {} blobs to be healed... Doing it",
                    healing_futures.len()
                );

                futures::stream::futures_unordered(healing_futures)
                    .collect()
                    .and_then(move |cleaned_entries: Vec<Vec<BlobstoreSyncQueueEntry>>| {
                        let cleaned = cleaned_entries.into_iter().flatten().collect();
                        cleanup_after_healing(ctx, sync_queue, cleaned)
                    })
            })
    }
}

/// Heal an individual blob. The `entries` are the blobstores which have successfully stored
/// this blob; we need to replicate them onto the remaining `blobstores`. If the blob is not
/// yet eligable (too young), then just return None, otherwise we return the healed entries
/// which have now been dealt with.
fn heal_blob(
    ctx: CoreContext,
    sync_queue: Arc<dyn BlobstoreSyncQueue>,
    blobstores: Arc<HashMap<BlobstoreId, Arc<dyn Blobstore>>>,
    healing_deadline: DateTime,
    key: String,
    entries: Vec<BlobstoreSyncQueueEntry>,
) -> Option<impl Future<Item = Vec<BlobstoreSyncQueueEntry>, Error = Error>> {
    let seen_blobstores: HashSet<_> = entries
        .iter()
        .filter_map(|entry| {
            let id = entry.blobstore_id;
            if blobstores.contains_key(&id) {
                Some(id)
            } else {
                None
            }
        })
        .collect();

    let mut stores_to_heal: HashSet<BlobstoreId> = blobstores
        .iter()
        .filter_map(|(key, _)| {
            if seen_blobstores.contains(key) {
                None
            } else {
                Some(key.clone())
            }
        })
        .collect();

    if stores_to_heal.is_empty() {
        // All blobstores have been synchronized
        return Some(futures::future::ok(entries).left_future());
    }

    if !entries
        .iter()
        .any(|entry| entry.timestamp < healing_deadline)
    {
        // The oldes entry is not old enough to be eligible for healing
        return None;
    }

    let heal_future = fetch_blob(
        ctx.clone(),
        blobstores.clone(),
        key.clone(),
        seen_blobstores.clone(),
    )
    .and_then(move |fetch_data| {
        if !fetch_data.missing_sources.is_empty() {
            warn!(
                ctx.logger(),
                "Source Blobstores {:?} of {:?} returned None even though they \
                 should contain data",
                fetch_data.missing_sources,
                seen_blobstores
            );
            for bid in fetch_data.missing_sources.clone() {
                stores_to_heal.insert(bid);
            }
        }

        let heal_puts: Vec<_> = stores_to_heal
            .into_iter()
            .map(|bid| {
                let blobstore = blobstores
                    .get(&bid)
                    .expect("stores_to_heal contains unknown blobstore?");
                blobstore
                    .put(ctx.clone(), key.clone(), fetch_data.blob.clone())
                    .then(move |result| Ok((bid, result.is_ok())))
            })
            .collect();

        // If any puts fail make sure we put a good source blobstore_id for that blob
        // back on the queue
        join_all(heal_puts).and_then(move |heal_results| {
            let (mut healed_stores, unhealed_stores): (HashSet<_>, HashSet<_>) =
                heal_results.into_iter().partition_map(|(id, put_ok)| {
                    if put_ok {
                        Either::Left(id)
                    } else {
                        Either::Right(id)
                    }
                });
            if !unhealed_stores.is_empty() {
                // Add good_sources to the healed_stores as we should write all
                // known good blobstores so that the stores_to_heal logic run on read
                // has the full data for the blobstore_key
                //
                // This also ensures we requeue at least one good source store in the case
                // where all heal puts fail
                for b in fetch_data.good_sources {
                    healed_stores.insert(b);
                }
                warn!(
                    ctx.logger(),
                    "Adding source blobstores {:?} to the queue so that failed \
                     destination blob stores {:?} will be retried later",
                    healed_stores,
                    unhealed_stores
                );
                requeue_partial_heal(ctx, sync_queue, key, healed_stores)
                    .map(|()| entries)
                    .left_future()
            } else {
                futures::future::ok(entries).right_future()
            }
        })
    });

    Some(heal_future.right_future())
}

struct FetchData {
    blob: BlobstoreBytes,
    good_sources: Vec<BlobstoreId>,
    missing_sources: Vec<BlobstoreId>,
}

/// Fetch a blob by `key` from one of the `seen_blobstores`. This tries them one at at time
/// sequentially, returning the known good store plus those found missing, or an error
fn fetch_blob(
    ctx: CoreContext,
    blobstores: Arc<HashMap<BlobstoreId, Arc<dyn Blobstore>>>,
    key: String,
    seen_blobstores: HashSet<BlobstoreId>,
) -> impl Future<Item = FetchData, Error = Error> {
    let err_context = format!(
        "While fetching blob '{}', seen in blobstores: {:?}",
        key, seen_blobstores
    );

    let get_futs: Vec<_> = seen_blobstores
        .iter()
        .cloned()
        .map(|bid| {
            let blobstore = blobstores
                .get(&bid)
                .expect("blobstores_to_fetch contains only existing blobstores");
            blobstore
                .get(ctx.clone(), key.clone())
                .then(move |result| Ok((bid, result)))
        })
        .collect();

    join_all(get_futs)
        .and_then(move |get_res| {
            let mut blob = None;
            let mut good_sources = vec![];
            let mut missing_sources = vec![];
            for (bid, r) in get_res {
                match r {
                    Ok(Some(blob_data)) => {
                        blob = Some(blob_data);
                        good_sources.push(bid);
                    }
                    Ok(None) => {
                        missing_sources.push(bid);
                    }
                    Err(e) => {
                        warn!(
                            ctx.logger(),
                            "error when loading from store {:?}: {:?}", bid, e
                        );
                    }
                }
            }
            match blob {
                None => futures::future::err(err_msg("None of the blobstores to fetch responded")),
                Some(blob_data) => futures::future::ok(FetchData {
                    blob: blob_data,
                    good_sources,
                    missing_sources,
                }),
            }
        })
        .chain_err(err_context)
        .from_err()
}

/// Removed healed entries from the queue.
fn cleanup_after_healing(
    ctx: CoreContext,
    sync_queue: Arc<dyn BlobstoreSyncQueue>,
    entries: Vec<BlobstoreSyncQueueEntry>,
) -> impl Future<Item = (), Error = Error> {
    sync_queue.del(ctx, entries)
}

/// Write new queue items with a populated source blobstore for unhealed entries
/// Uses a current timestamp so we'll get around to trying them again for the destination
/// blobstores eventually without getting stuck on them permanently.
/// Uses a new queue entry id so the delete of original entry is safe.
fn requeue_partial_heal(
    ctx: CoreContext,
    sync_queue: Arc<dyn BlobstoreSyncQueue>,
    blobstore_key: String,
    source_blobstores: impl IntoIterator<Item = BlobstoreId>,
) -> impl Future<Item = (), Error = Error> {
    let timestamp = DateTime::now();

    join_all(source_blobstores.into_iter().map({
        move |blobstore_id| {
            cloned!(ctx, blobstore_key, timestamp);
            sync_queue.add(
                ctx,
                BlobstoreSyncQueueEntry {
                    blobstore_key,
                    blobstore_id,
                    timestamp,
                    id: None,
                },
            )
        }
    }))
    .map(|_: Vec<()>| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::Future;
    use futures_ext::BoxFuture;
    use std::iter::FromIterator;
    use std::sync::{Mutex, RwLock};

    // In-memory "blob store"
    ///
    /// Pure in-memory implementation for testing, with put failure
    #[derive(Clone, Debug)]
    pub struct PutFailingEagerMemblob {
        hash: Arc<Mutex<HashMap<String, BlobstoreBytes>>>,
        fail_puts: Arc<Mutex<bool>>,
    }

    impl PutFailingEagerMemblob {
        pub fn new() -> Self {
            Self {
                hash: Arc::new(Mutex::new(HashMap::new())),
                fail_puts: Arc::new(Mutex::new(false)),
            }
        }
        pub fn len(&self) -> usize {
            let inner = self.hash.lock().expect("lock poison");
            inner.len()
        }
        pub fn fail_puts(&self) {
            let mut data = self.fail_puts.lock().expect("lock poison");
            *data = true;
        }
    }

    impl Blobstore for PutFailingEagerMemblob {
        fn put(
            &self,
            _ctx: CoreContext,
            key: String,
            value: BlobstoreBytes,
        ) -> BoxFuture<(), Error> {
            let mut inner = self.hash.lock().expect("lock poison");
            let inner_flag = self.fail_puts.lock().expect("lock poison");
            let res = if *inner_flag {
                Err(err_msg("Put failed for key"))
            } else {
                inner.insert(key, value);
                Ok(())
            };
            res.into_future().boxify()
        }

        fn get(&self, _ctx: CoreContext, key: String) -> BoxFuture<Option<BlobstoreBytes>, Error> {
            let inner = self.hash.lock().expect("lock poison");
            Ok(inner.get(&key).map(Clone::clone)).into_future().boxify()
        }
    }

    pub struct MockBlobstoreSyncQueue {
        queue: RwLock<Vec<BlobstoreSyncQueueEntry>>,
    }

    impl MockBlobstoreSyncQueue {
        fn new() -> Self {
            Self {
                queue: RwLock::new(Vec::new()),
            }
        }
        fn len(&self) -> usize {
            self.queue.read().unwrap().len()
        }
    }

    impl BlobstoreSyncQueue for MockBlobstoreSyncQueue {
        fn add_many(
            &self,
            _ctx: CoreContext,
            entries: Box<dyn Iterator<Item = BlobstoreSyncQueueEntry> + Send>,
        ) -> BoxFuture<(), Error> {
            for e in entries {
                self.queue.write().unwrap().push(e);
            }
            futures::future::ok(()).boxify()
        }

        fn iter(
            &self,
            _ctx: CoreContext,
            _key_like: Option<String>,
            _older_than: DateTime,
            _limit: usize,
        ) -> BoxFuture<Vec<BlobstoreSyncQueueEntry>, Error> {
            unimplemented!();
        }

        fn del(
            &self,
            _ctx: CoreContext,
            entries: Vec<BlobstoreSyncQueueEntry>,
        ) -> BoxFuture<(), Error> {
            let delhash: HashSet<_> = HashSet::from_iter(entries.iter().map(|e| e.id));
            self.queue
                .write()
                .unwrap()
                .retain(|e| delhash.contains(&e.id));
            futures::future::ok(()).boxify()
        }

        fn get(
            &self,
            _ctx: CoreContext,
            _key: String,
        ) -> BoxFuture<Vec<BlobstoreSyncQueueEntry>, Error> {
            // TODO, see if we can remove this method from the trait
            unimplemented!();
        }
    }

    fn make_empty_stores(
        n: usize,
    ) -> (
        Vec<BlobstoreId>,
        HashMap<BlobstoreId, Arc<PutFailingEagerMemblob>>,
        Arc<HashMap<BlobstoreId, Arc<dyn Blobstore>>>,
    ) {
        let mut test_bids = Vec::new();
        let mut test_stores = HashMap::new();
        let mut underlying_stores = HashMap::new();
        for i in 0..n {
            test_bids.push(BlobstoreId::new(i as u64));
            let u = Arc::new(PutFailingEagerMemblob::new());
            let s: Arc<dyn Blobstore> = u.clone();
            test_stores.insert(test_bids[i], s);
            underlying_stores.insert(test_bids[i], u);
        }
        let stores = Arc::new(test_stores);
        // stores loses its concrete typing, so return underlying to allow access to len() etc.
        (test_bids, underlying_stores, stores)
    }

    fn make_value(value: &str) -> BlobstoreBytes {
        BlobstoreBytes::from_bytes(value.as_bytes())
    }

    fn put_value(ctx: &CoreContext, store: Option<&Arc<dyn Blobstore>>, key: &str, value: &str) {
        store.map(|s| s.put(ctx.clone(), key.to_string(), make_value(value)));
    }

    #[test]
    fn fetch_blob_missing_all() {
        let ctx = CoreContext::test_mock();
        let (bids, _underlying_stores, stores) = make_empty_stores(3);
        let fut = fetch_blob(
            ctx,
            stores,
            "specialk".to_string(),
            HashSet::from_iter(bids.into_iter()),
        );
        let r = fut.wait();
        let msg = r
            .err()
            .and_then(|e| e.as_fail().cause().map(|f| (format!("{}", f))));
        assert_eq!(
            Some("None of the blobstores to fetch responded".to_string()),
            msg
        );
    }

    #[test]
    fn heal_blob_missing_all_stores() {
        let ctx = CoreContext::test_mock();
        let (bids, underlying_stores, stores) = make_empty_stores(3);
        let healing_deadline = DateTime::from_rfc3339("2019-07-01T12:00:00.00Z").unwrap();
        let t0 = DateTime::from_rfc3339("2018-11-29T12:00:00.00Z").unwrap();
        let entries = vec![
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[0], t0),
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[1], t0),
        ];
        let sync_queue = Arc::new(MockBlobstoreSyncQueue::new());
        let fut = heal_blob(
            ctx,
            sync_queue.clone(),
            stores.clone(),
            healing_deadline,
            "specialk".to_string(),
            entries,
        );
        let r = fut.wait();
        let msg = r
            .err()
            .and_then(|e| e.as_fail().cause().map(|f| (format!("{}", f))));
        assert_eq!(
            Some("None of the blobstores to fetch responded".to_string()),
            msg
        );
        assert_eq!(
            0,
            sync_queue.len(),
            "Should be nothing on queue as deletion step won't run"
        );
        assert_eq!(
            0,
            underlying_stores.get(&bids[0]).unwrap().len(),
            "Should still be empty as no healing possible"
        );
        assert_eq!(
            0,
            underlying_stores.get(&bids[1]).unwrap().len(),
            "Should still be empty as no healing possible"
        );
        assert_eq!(
            0,
            underlying_stores.get(&bids[2]).unwrap().len(),
            "Should still be empty as no healing possible"
        );
    }

    #[test]
    fn heal_blob_where_queue_and_stores_match_on_missing() {
        let ctx = CoreContext::test_mock();
        let (bids, underlying_stores, stores) = make_empty_stores(3);
        put_value(&ctx, stores.get(&bids[0]), "specialk", "specialv");
        put_value(&ctx, stores.get(&bids[1]), "specialk", "specialv");
        put_value(&ctx, stores.get(&bids[2]), "dummyk", "dummyv");
        let healing_deadline = DateTime::from_rfc3339("2019-07-01T12:00:00.00Z").unwrap();
        let t0 = DateTime::from_rfc3339("2018-11-29T12:00:00.00Z").unwrap();
        let entries = vec![
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[0], t0),
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[1], t0),
        ];
        let sync_queue = Arc::new(MockBlobstoreSyncQueue::new());
        let fut = heal_blob(
            ctx,
            sync_queue.clone(),
            stores.clone(),
            healing_deadline,
            "specialk".to_string(),
            entries,
        );
        let r = fut.wait();
        assert!(r.is_ok());
        assert_eq!(
            2,
            r.unwrap().unwrap().len(),
            "expecting 2 entries to delete"
        );
        assert_eq!(1, underlying_stores.get(&bids[0]).unwrap().len());
        assert_eq!(1, underlying_stores.get(&bids[1]).unwrap().len());
        assert_eq!(
            2,
            underlying_stores.get(&bids[2]).unwrap().len(),
            "Expected extra entry after heal"
        );
        assert_eq!(
            0,
            sync_queue.len(),
            "expecting 0 entries to write to queue for reheal as we just healed the last one"
        );
    }

    #[test]
    fn fetch_blob_missing_none() {
        let ctx = CoreContext::test_mock();
        let (bids, _underlying_stores, stores) = make_empty_stores(3);
        put_value(&ctx, stores.get(&bids[0]), "specialk", "specialv");
        put_value(&ctx, stores.get(&bids[1]), "specialk", "specialv");
        put_value(&ctx, stores.get(&bids[2]), "specialk", "specialv");
        let fut = fetch_blob(
            ctx,
            stores,
            "specialk".to_string(),
            HashSet::from_iter(bids.into_iter()),
        );
        let r = fut.wait();
        let foundv = r.ok().unwrap().blob;
        assert_eq!(make_value("specialv"), foundv);
    }

    #[test]
    fn heal_blob_missing_none() {
        let ctx = CoreContext::test_mock();
        let (bids, underlying_stores, stores) = make_empty_stores(3);
        put_value(&ctx, stores.get(&bids[0]), "specialk", "specialv");
        put_value(&ctx, stores.get(&bids[1]), "specialk", "specialv");
        put_value(&ctx, stores.get(&bids[2]), "specialk", "specialv");
        let healing_deadline = DateTime::from_rfc3339("2019-07-01T12:00:00.00Z").unwrap();
        let t0 = DateTime::from_rfc3339("2018-11-29T12:00:00.00Z").unwrap();
        let entries = vec![
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[0], t0),
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[1], t0),
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[2], t0),
        ];
        let sync_queue = Arc::new(MockBlobstoreSyncQueue::new());
        let fut = heal_blob(
            ctx,
            sync_queue.clone(),
            stores,
            healing_deadline,
            "specialk".to_string(),
            entries,
        );
        let r = fut.wait();
        assert!(r.is_ok());
        assert_eq!(
            3,
            r.unwrap().unwrap().len(),
            "expecting 3 entries to delete"
        );
        assert_eq!(0, sync_queue.len());
        assert_eq!(1, underlying_stores.get(&bids[0]).unwrap().len());
        assert_eq!(1, underlying_stores.get(&bids[1]).unwrap().len());
        assert_eq!(1, underlying_stores.get(&bids[2]).unwrap().len());
    }

    #[test]
    fn fetch_blob_missing_some() {
        let ctx = CoreContext::test_mock();
        let (bids, _underlying_stores, stores) = make_empty_stores(3);
        put_value(&ctx, stores.get(&bids[0]), "specialk", "specialv");
        let fut = fetch_blob(
            ctx,
            stores,
            "specialk".to_string(),
            HashSet::from_iter(bids.clone().into_iter()),
        );
        let r = fut.wait();
        let mut fetch_data: FetchData = r.ok().unwrap();
        assert_eq!(make_value("specialv"), fetch_data.blob);
        fetch_data.good_sources.sort();
        assert_eq!(fetch_data.good_sources, &bids[0..1]);
        fetch_data.missing_sources.sort();
        assert_eq!(fetch_data.missing_sources, &bids[1..3]);
    }

    #[test]
    fn heal_blob_where_queue_and_stores_mismatch_on_missing() {
        let ctx = CoreContext::test_mock();
        let (bids, underlying_stores, stores) = make_empty_stores(3);
        put_value(&ctx, stores.get(&bids[0]), "specialk", "specialv");
        put_value(&ctx, stores.get(&bids[1]), "specialk", "specialv");
        put_value(&ctx, stores.get(&bids[2]), "dummyk", "dummyv");
        let healing_deadline = DateTime::from_rfc3339("2019-07-01T12:00:00.00Z").unwrap();
        let t0 = DateTime::from_rfc3339("2018-11-29T12:00:00.00Z").unwrap();
        let entries = vec![
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[0], t0),
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[2], t0),
        ];
        let sync_queue = Arc::new(MockBlobstoreSyncQueue::new());
        let fut = heal_blob(
            ctx,
            sync_queue.clone(),
            stores.clone(),
            healing_deadline,
            "specialk".to_string(),
            entries,
        );
        let r = fut.wait();
        assert!(r.is_ok());
        assert_eq!(
            2,
            r.unwrap().unwrap().len(),
            "expecting 2 entries to delete"
        );
        assert_eq!(1, underlying_stores.get(&bids[0]).unwrap().len());
        assert_eq!(
            1,
            underlying_stores.get(&bids[1]).unwrap().len(),
            "Expected same entry after heal despite bad queue"
        );
        assert_eq!(
            2,
            underlying_stores.get(&bids[2]).unwrap().len(),
            "Expected extra entry after heal"
        );
        assert_eq!(
            0,
            sync_queue.len(),
            "expecting 0 entries to write to queue for reheal as all heal puts succeeded"
        );
    }

    #[test]
    fn heal_blob_where_store_and_queue_match_all_put_fails() {
        let ctx = CoreContext::test_mock();
        let (bids, underlying_stores, stores) = make_empty_stores(3);
        put_value(&ctx, stores.get(&bids[0]), "specialk", "specialv");
        put_value(&ctx, stores.get(&bids[1]), "specialk", "specialv");
        put_value(&ctx, stores.get(&bids[2]), "dummyk", "dummyv");
        let healing_deadline = DateTime::from_rfc3339("2019-07-01T12:00:00.00Z").unwrap();
        let t0 = DateTime::from_rfc3339("2018-11-29T12:00:00.00Z").unwrap();
        let entries = vec![
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[0], t0),
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[1], t0),
        ];
        underlying_stores.get(&bids[2]).unwrap().fail_puts();
        let sync_queue = Arc::new(MockBlobstoreSyncQueue::new());
        let fut = heal_blob(
            ctx,
            sync_queue.clone(),
            stores.clone(),
            healing_deadline,
            "specialk".to_string(),
            entries,
        );
        let r = fut.wait();
        assert!(r.is_ok());
        assert_eq!(
            2,
            r.unwrap().unwrap().len(),
            "expecting 2 entries to delete"
        );
        assert_eq!(1, underlying_stores.get(&bids[0]).unwrap().len());
        assert_eq!(
            1,
            underlying_stores.get(&bids[0]).unwrap().len(),
            "Expected same entry after heal e"
        );
        assert_eq!(
            1,
            underlying_stores.get(&bids[1]).unwrap().len(),
            "Expected same entry after heal"
        );
        assert_eq!(
            1,
            underlying_stores.get(&bids[2]).unwrap().len(),
            "Expected same entry after heal due to put failure"
        );
        assert_eq!(
            2,
            sync_queue.len(),
            "expecting 2 known good entries to write to queue for reheal as there was a put failure"
        );
    }

    #[test]
    fn heal_blob_where_store_and_queue_mismatch_some_put_fails() {
        let ctx = CoreContext::test_mock();
        let (bids, underlying_stores, stores) = make_empty_stores(3);
        put_value(&ctx, stores.get(&bids[0]), "specialk", "specialv");
        put_value(&ctx, stores.get(&bids[1]), "dummyk", "dummyk");
        put_value(&ctx, stores.get(&bids[2]), "dummyk", "dummyv");
        let healing_deadline = DateTime::from_rfc3339("2019-07-01T12:00:00.00Z").unwrap();
        let t0 = DateTime::from_rfc3339("2018-11-29T12:00:00.00Z").unwrap();
        let entries = vec![
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[0], t0),
            BlobstoreSyncQueueEntry::new("specialk".to_string(), bids[1], t0),
        ];
        underlying_stores.get(&bids[1]).unwrap().fail_puts();
        let sync_queue = Arc::new(MockBlobstoreSyncQueue::new());
        let fut = heal_blob(
            ctx,
            sync_queue.clone(),
            stores.clone(),
            healing_deadline,
            "specialk".to_string(),
            entries,
        );
        let r = fut.wait();
        assert!(r.is_ok());
        assert_eq!(
            2,
            r.unwrap().unwrap().len(),
            "expecting 2 entries to delete"
        );
        assert_eq!(1, underlying_stores.get(&bids[0]).unwrap().len());
        assert_eq!(
            1,
            underlying_stores.get(&bids[0]).unwrap().len(),
            "Expected same entry after heal e"
        );
        assert_eq!(
            1,
            underlying_stores.get(&bids[1]).unwrap().len(),
            "Expected same after heal as put fail prevents heal"
        );
        assert_eq!(
            2,
            underlying_stores.get(&bids[2]).unwrap().len(),
            "Expected extra entry after heal"
        );
        assert_eq!(
            2,
            sync_queue.len(),
            "expecting 2 known good entries to write to queue for reheal as there was a put failure"
        );
    }
}
