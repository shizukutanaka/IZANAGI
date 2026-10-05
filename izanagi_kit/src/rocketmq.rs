//! RocketMQ ブローカー設定(`broker.conf`)の検出と構造カウント。
//!
//! `brokerClusterName`/`brokerName`/`brokerId`/`brokerRole`/`namesrvAddr`/
//! `listenPort`/`storePath*`/`flushDiskType`/`autoCreateTopicEnable`/
//! `dLeger*` 等の既知 `key=value` プロパティを識別する。
//!
//! ```
//! let c = izanagi_kit::rocketmq::parse(
//!     b"brokerClusterName=DefaultCluster\nbrokerName=broker-a\nbrokerId=0\nnamesrvAddr=127.0.0.1:9876\n").unwrap();
//! assert_eq!(c.options, 4);
//! assert!(izanagi_kit::rocketmq::detect(b"brokerName=broker-a\nbrokerRole=ASYNC_MASTER\nflushDiskType=ASYNC_FLUSH\n"));
//! ```

/// 既知プロパティキー。
const KEYS: &[&str] = &[
    "aclEnable",
    "autoCreateSubscriptionGroup",
    "autoCreateTopicEnable",
    "brokerBusyCheckTimeMillis",
    "brokerClusterName",
    "brokerFastFailureEnable",
    "brokerId",
    "brokerIP1",
    "brokerIP2",
    "brokerMemberGroup",
    "brokerName",
    "brokerNotBusyTimeoutMills",
    "brokerPermission",
    "brokerRole",
    "brokerTopicEnable",
    "cleanFileForciblyEnable",
    "cleanResourceInterval",
    "commitIntervalCommitLog",
    "commitLogMaxDepositedSlotGroup",
    "commitOffsetTableSize",
    "commitPeriodical",
    "compressMsgBodyOverHowmuch",
    "consumerFallbehindThreshold",
    "consumerManageThreadPoolNums",
    "dLegerGroup",
    "dLegerPeers",
    "dLegerSelfId",
    "defaultMessageStorePlugIn",
    "defaultTopicQueueNums",
    "deleteWhen",
    "diskMaxUsedSpaceRatio",
    "duplicationEnable",
    "enableCalcFilterBitMap",
    "enableConsumeQueueExt",
    "enableControllerMode",
    "enableDLegerCommitLog",
    "enablePropertyFilter",
    "enableSlaveActingMaster",
    "enableTransientStorePool",
    "expectConsumerNumUseFilter",
    "fastFailIfNoBufferInStorePool",
    "fetchNamesrvAddrByAddressServer",
    "fileReservedTime",
    "filterDataRepartitionRatio",
    "filterServerNums",
    "flushCommitLogLeastPages",
    "flushCommitLogThoroughInterval",
    "flushConsumerOffsetInterval",
    "flushConsumerOffsetLeastPages",
    "flushConsumerOffsetThoroughInterval",
    "flushDelayOffsetInterval",
    "flushDiskType",
    "flushIntervalCommitLog",
    "flushIntervalConsumeQueue",
    "forceReject",
    "haHousekeepingInterval",
    "haListenPort",
    "haMasterAddress",
    "haTransferBatchSize",
    "haSlaveFallBehindMax",
    "listenPort",
    "longPollingEnable",
    "mappedFileSizeCommitLog",
    "mapedFileSizeCommitLog",
    "mapedFileSizeConsumeQueue",
    "mappedFileSizeConsumeQueue",
    "maxErrorRateOfBloomFilter",
    "maxFilterBitMap",
    "maxHashSlotNum",
    "maxMessageSize",
    "maxMsgsNumBatch",
    "maxTransferBytesOnMessageInMemory",
    "maxTransferBytesOnMessageInDisk",
    "maxTransferCountOnMessageInMemory",
    "maxTransferCountOnMessageInDisk",
    "messageDelayLevel",
    "messageIndexEnable",
    "messageIndexSafe",
    "messageRetrievePlugIn",
    "messageStorePlugIn",
    "namesrvAddr",
    "notifyConsumerIdsChangedEnable",
    "osPageCacheBusyTimeOutMills",
    "pullMessageThreadPoolNums",
    "putMsgAverageSize",
    "queryMessageThreadPoolNums",
    "recoverConcurrently",
    "registerBrokerTimeoutMills",
    "rejectTransactionMessage",
    "rocketmqHome",
    "sendMessageThreadPoolNums",
    "sendMessageWithVIPChannel",
    "serverAsyncSelectorThreads",
    "serverCallbackExecutorThreads",
    "serverChannelMaxIdleTimeSeconds",
    "serverOnewaySemaphoreValue",
    "serverPooledByteBufAllocatorEnable",
    "serverSelectorThreads",
    "serverSocketRcvBufSize",
    "serverSocketSndBufSize",
    "serverWorkerThreads",
    "shortPollingTimeMills",
    "slaveReadEnable",
    "startAcceptSendRequestTimeStamp",
    "storeCheckpoint",
    "storePathCommitLog",
    "storePathConsumeQueue",
    "storePathConsumerQueue",
    "storePathEpochFile",
    "storePathIndex",
    "storePathRootDir",
    "syncFlushTimeout",
    "transactionCheckInterval",
    "transactionCheckMax",
    "transactionCheckTimePollMax",
    "transactionMsgTimeout",
    "transactionTimeOut",
    "transferMsgByHeap",
    "transientStorePoolSize",
    "useEpollNativeSelector",
    "waitTimeMillsInPullQueue",
    "waitTimeMillsInSendQueue",
    "waitTimeMillsInTransactionQueue",
    "warmMapedFileEnable",
];

/// rocketmq 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知プロパティ行。
    pub options: usize,
    /// `#`/`;` コメント行。
    pub comments: usize,
    /// 分類不能行(未知キー・その他)。
    pub misc: usize,
}

/// `key` 区切り(`=`/`:`)前のキー名。
fn kv_key(t: &str) -> Option<&str> {
    let pe = t.find('=');
    let pc = t.find(':');
    let p = match (pe, pc) {
        (Some(e), Some(co)) => e.min(co),
        (Some(e), None) => e,
        (None, Some(co)) => co,
        (None, None) => return None,
    };
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        None
    } else {
        Some(k)
    }
}

/// b が broker.conf かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && !t.starts_with('#')
                && !t.starts_with(';')
                && kv_key(t).is_some_and(|k| KEYS.contains(&k))
        })
        .count()
        >= 3
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if kv_key(t).is_some_and(|k| KEYS.contains(&k)) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# rocketmq broker\nbrokerClusterName=DefaultCluster\nbrokerName=broker-a\nbrokerId=0\nbrokerRole=ASYNC_MASTER\nflushDiskType=ASYNC_FLUSH\nnamesrvAddr=127.0.0.1:9876\nlistenPort=10911\nstorePathRootDir=/data/store\nstorePathCommitLog=/data/store/commitlog\nautoCreateTopicEnable=true\nmappedFileSizeCommitLog=1073741824\ndeleteWhen=04\nfileReservedTime=48\n";

    #[test]
    fn rocketmq() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 13);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_rocketmq() {
        assert!(!detect(b"key=value\nother=thing\n"));
        assert!(!detect(b"brokerName=x\n"));
    }
}
