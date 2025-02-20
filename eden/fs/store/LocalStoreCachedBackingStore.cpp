/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#include "eden/fs/store/LocalStoreCachedBackingStore.h"
#include "eden/fs/model/Blob.h"
#include "eden/fs/model/Tree.h"
#include "eden/fs/store/LocalStore.h"
#include "eden/fs/telemetry/EdenStats.h"
#include "eden/fs/utils/ImmediateFuture.h"

namespace facebook::eden {

LocalStoreCachedBackingStore::LocalStoreCachedBackingStore(
    std::shared_ptr<BackingStore> backingStore,
    std::shared_ptr<LocalStore> localStore,
    EdenStatsPtr stats)
    : backingStore_{std::move(backingStore)},
      localStore_{std::move(localStore)},
      stats_{std::move(stats)} {}

LocalStoreCachedBackingStore::~LocalStoreCachedBackingStore() {}

ObjectComparison LocalStoreCachedBackingStore::compareObjectsById(
    const ObjectId& one,
    const ObjectId& two) {
  return backingStore_->compareObjectsById(one, two);
}

ImmediateFuture<std::unique_ptr<Tree>>
LocalStoreCachedBackingStore::getRootTree(
    const RootId& rootId,
    const ObjectFetchContextPtr& context) {
  return backingStore_->getRootTree(rootId, context)
      .thenValue([localStore = localStore_](std::unique_ptr<Tree> tree) {
        // TODO: perhaps this callback should use toUnsafeFuture() to ensure the
        // tree is cached whether or not the caller consumes the future.
        if (tree) {
          localStore->putTree(*tree);
        }
        return tree;
      });
}

ImmediateFuture<std::unique_ptr<TreeEntry>>
LocalStoreCachedBackingStore::getTreeEntryForObjectId(
    const ObjectId& objectId,
    TreeEntryType treeEntryType,
    const ObjectFetchContextPtr& context) {
  return backingStore_->getTreeEntryForObjectId(
      objectId, treeEntryType, context);
}

folly::SemiFuture<BackingStore::GetTreeResult>
LocalStoreCachedBackingStore::getTree(
    const ObjectId& id,
    const ObjectFetchContextPtr& context) {
  return localStore_->getTree(id)
      .thenValue([id = id,
                  context = context.copy(),
                  localStore = localStore_,
                  backingStore =
                      backingStore_](std::unique_ptr<Tree> tree) mutable {
        if (tree) {
          return folly::makeSemiFuture(GetTreeResult{
              std::move(tree), ObjectFetchContext::FromDiskCache});
        }

        return backingStore
            ->getTree(id, context)
            // TODO: This is a good use for toUnsafeFuture to ensure the tree is
            // cached even if the resulting future is never consumed.
            .deferValue(
                [localStore = std::move(localStore)](GetTreeResult result) {
                  if (result.tree) {
                    auto batch = localStore->beginWrite();
                    batch->putTree(*result.tree);

                    // Let's cache all the entries in the LocalStore.
                    for (const auto& [name, treeEntry] : *result.tree) {
                      const auto& size = treeEntry.getSize();
                      const auto& sha1 = treeEntry.getContentSha1();
                      if (treeEntry.getType() == TreeEntryType::REGULAR_FILE &&
                          size && sha1) {
                        batch->putBlobMetadata(
                            treeEntry.getHash(), BlobMetadata{*sha1, *size});
                      }
                    }
                    batch->flush();
                  }

                  return result;
                });
      })
      .semi();
}

folly::SemiFuture<BackingStore::GetBlobMetaResult>
LocalStoreCachedBackingStore::getBlobMetadata(
    const ObjectId& id,
    const ObjectFetchContextPtr& context) {
  return localStore_->getBlobMetadata(id)
      .thenValue([self = shared_from_this(), id = id, context = context.copy()](
                     std::unique_ptr<BlobMetadata> metadata) mutable {
        if (metadata) {
          self->stats_->increment(
              &ObjectStoreStats::getBlobMetadataFromLocalStore);
          return folly::makeSemiFuture(GetBlobMetaResult{
              std::move(metadata), ObjectFetchContext::FromDiskCache});
        }

        return self->backingStore_->getBlobMetadata(id, context)
            .deferValue(
                [self, id, context = context.copy()](GetBlobMetaResult result)
                    -> folly::SemiFuture<GetBlobMetaResult> {
                  if (result.blobMeta) {
                    if (result.origin ==
                        ObjectFetchContext::Origin::FromDiskCache) {
                      self->stats_->increment(
                          &ObjectStoreStats::
                              getLocalBlobMetadataFromBackingStore);
                    } else {
                      self->stats_->increment(
                          &ObjectStoreStats::getBlobMetadataFromBackingStore);
                    }

                    return result;
                  }

                  return self->getBlob(id, context)
                      .deferValue([self](GetBlobResult result) {
                        if (result.blob) {
                          self->stats_->increment(
                              &ObjectStoreStats::getBlobMetadataFromBlob);
                        }

                        return GetBlobMetaResult{
                            std::make_unique<BlobMetadata>(
                                Hash20::sha1(result.blob->getContents()),
                                result.blob->getSize()),
                            result.origin};
                      });
                })
            .deferValue(
                [localStore = self->localStore_, id](GetBlobMetaResult result) {
                  if (result.blobMeta) {
                    localStore->putBlobMetadata(id, *result.blobMeta);
                  }
                  return result;
                });
      })
      .semi();
}

folly::SemiFuture<BackingStore::GetBlobResult>
LocalStoreCachedBackingStore::getBlob(
    const ObjectId& id,
    const ObjectFetchContextPtr& context) {
  return localStore_->getBlob(id)
      .thenValue([id = id,
                  context = context.copy(),
                  localStore = localStore_,
                  backingStore = backingStore_,
                  stats = stats_.copy()](std::unique_ptr<Blob> blob) mutable {
        if (blob) {
          stats->increment(&ObjectStoreStats::getBlobFromLocalStore);
          return folly::makeSemiFuture(GetBlobResult{
              std::move(blob), ObjectFetchContext::FromDiskCache});
        }

        return backingStore
            ->getBlob(id, context)
            // TODO: This is a good use for toUnsafeFuture to ensure the tree is
            // cached even if the resulting future is never consumed.
            .deferValue([localStore = std::move(localStore),
                         stats = std::move(stats),
                         id](GetBlobResult result) {
              if (result.blob) {
                localStore->putBlob(id, result.blob.get());
                stats->increment(&ObjectStoreStats::getBlobFromBackingStore);
              }
              return result;
            });
      })
      .semi();
}

folly::SemiFuture<folly::Unit> LocalStoreCachedBackingStore::prefetchBlobs(
    ObjectIdRange ids,
    const ObjectFetchContextPtr& context) {
  return backingStore_->prefetchBlobs(ids, context);
}

void LocalStoreCachedBackingStore::periodicManagementTask() {
  backingStore_->periodicManagementTask();
}

void LocalStoreCachedBackingStore::startRecordingFetch() {
  backingStore_->startRecordingFetch();
}

std::unordered_set<std::string>
LocalStoreCachedBackingStore::stopRecordingFetch() {
  return backingStore_->stopRecordingFetch();
}

folly::SemiFuture<folly::Unit>
LocalStoreCachedBackingStore::importManifestForRoot(
    const RootId& rootId,
    const Hash20& manifest) {
  return backingStore_->importManifestForRoot(rootId, manifest);
}

RootId LocalStoreCachedBackingStore::parseRootId(folly::StringPiece rootId) {
  return backingStore_->parseRootId(rootId);
}

std::string LocalStoreCachedBackingStore::renderRootId(const RootId& rootId) {
  return backingStore_->renderRootId(rootId);
}

ObjectId LocalStoreCachedBackingStore::parseObjectId(
    folly::StringPiece objectId) {
  return backingStore_->parseObjectId(objectId);
}

std::string LocalStoreCachedBackingStore::renderObjectId(
    const ObjectId& objectId) {
  return backingStore_->renderObjectId(objectId);
}

std::optional<folly::StringPiece> LocalStoreCachedBackingStore::getRepoName() {
  return backingStore_->getRepoName();
}

} // namespace facebook::eden
