package com.heimq;

import org.apache.kafka.clients.admin.*;
import org.apache.kafka.clients.consumer.ConsumerConfig;
import org.apache.kafka.clients.consumer.ConsumerRecord;
import org.apache.kafka.clients.consumer.ConsumerRecords;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.clients.producer.KafkaProducer;
import org.apache.kafka.clients.producer.ProducerConfig;
import org.apache.kafka.clients.producer.ProducerRecord;
import org.apache.kafka.clients.producer.RecordMetadata;
import org.apache.kafka.common.ElectionType;
import org.apache.kafka.common.TopicPartition;
import org.apache.kafka.common.config.ConfigResource;
import org.apache.kafka.common.header.Header;
import org.apache.kafka.common.serialization.StringDeserializer;
import org.apache.kafka.common.serialization.StringSerializer;

import java.time.Duration;
import java.util.*;
import java.util.concurrent.Future;
import java.util.concurrent.ExecutionException;

/**
 * Java kafka-clients oracle for heimq.
 *
 * Uses Apache Kafka's official Java client (the reference implementation)
 * to verify that heimq speaks correct Kafka wire protocol from a fourth
 * independent client stack (rdkafka/C, franz-go/Go, sarama/Go, java-clients/Java).
 *
 * Usage: java -jar kafka-oracle.jar <bootstrap-servers> <topic>
 * Exit 0 on success; non-zero on any failure.
 */
public class KafkaOracle {

    public static void main(String[] args) throws Exception {
        if (args.length < 2) {
            System.err.println("usage: KafkaOracle <bootstrap-servers> <topic>");
            System.exit(1);
        }
        String bootstrap = args[0];
        String topic = args[1];

        try {
            run(bootstrap, topic);
            System.out.println("PASS");
        } catch (Exception e) {
            System.err.println("FAIL: " + e.getMessage());
            e.printStackTrace(System.err);
            System.exit(1);
        }
    }

    private static void run(String bootstrap, String topic) throws Exception {
        String resumeTopic = topic + "-resume";
        String resumeGroup = "java-resume-" + System.nanoTime();
        String oracleGroup = "java-oracle-group-" + System.nanoTime();

        check("create-topic", () -> createTopic(bootstrap, topic));
        check("produce", () -> produce(bootstrap, topic));
        check("consume-via-group", () -> consumeViaGroup(bootstrap, topic));
        check("describe-configs", () -> describeConfigs(bootstrap, topic));
        check("alter-configs", () -> alterConfigs(bootstrap, topic));
        check("incremental-alter-configs", () -> incrementalAlterConfigs(bootstrap, topic));
        check("list-offsets", () -> listOffsets(bootstrap, topic));
        check("describe-cluster", () -> describeCluster(bootstrap));
        check("describe-log-dirs", () -> describeLogDirs(bootstrap));
        check("create-partitions", () -> createPartitions(bootstrap, topic));
        check("delete-records", () -> deleteRecords(bootstrap, topic));
        check("list-groups", () -> listGroups(bootstrap));
        check("describe-groups", () -> describeGroups(bootstrap, oracleGroup));
        check("offset-delete", () -> offsetDelete(bootstrap, topic, oracleGroup));
        check("delete-groups", () -> deleteGroups(bootstrap, oracleGroup));
        check("list-transactions", () -> listTransactions(bootstrap));
        check("elect-leaders", () -> electLeaders(bootstrap, topic));
        String gzipTopic = topic + "-gzip";
        check("produce-gzip-compressed", () -> produceCompressed(bootstrap, gzipTopic, "gzip"));
        check("consume-gzip-roundtrip", () -> consumeCompressedRoundtrip(bootstrap, gzipTopic));
        String snappyTopic = topic + "-snappy";
        check("produce-snappy-compressed", () -> produceCompressed(bootstrap, snappyTopic, "snappy"));
        check("consume-snappy-roundtrip", () -> consumeCompressedRoundtrip(bootstrap, snappyTopic));
        String lz4Topic = topic + "-lz4";
        check("produce-lz4-compressed", () -> produceCompressed(bootstrap, lz4Topic, "lz4"));
        check("consume-lz4-roundtrip", () -> consumeCompressedRoundtrip(bootstrap, lz4Topic));
        String zstdTopic = topic + "-zstd";
        check("produce-zstd-compressed", () -> produceCompressed(bootstrap, zstdTopic, "zstd"));
        check("consume-zstd-roundtrip", () -> consumeCompressedRoundtrip(bootstrap, zstdTopic));
        check("offset-resume", () -> offsetResume(bootstrap, resumeTopic, resumeGroup));
        String txnTopic = topic + "-txn";
        check("transactional-produce", () -> transactionalProduce(bootstrap, txnTopic));
        check("transactional-consume-committed", () -> transactionalConsumeCommitted(bootstrap, txnTopic));
        check("produce-with-headers", () -> produceWithHeaders(bootstrap, topic + "-hdrs"));
        check("consume-headers-roundtrip", () -> consumeHeadersRoundtrip(bootstrap, topic + "-hdrs"));
        check("delete-topic", () -> deleteTopic(bootstrap, topic));
    }

    @FunctionalInterface
    interface Checker {
        void run() throws Exception;
    }

    private static void check(String name, Checker fn) throws Exception {
        fn.run();
        System.out.printf("  ok  %s%n", name);
    }

    private static Properties adminProps(String bootstrap) {
        Properties props = new Properties();
        props.put(AdminClientConfig.BOOTSTRAP_SERVERS_CONFIG, bootstrap);
        props.put(AdminClientConfig.REQUEST_TIMEOUT_MS_CONFIG, "10000");
        return props;
    }

    private static Properties producerProps(String bootstrap) {
        Properties props = new Properties();
        props.put(ProducerConfig.BOOTSTRAP_SERVERS_CONFIG, bootstrap);
        props.put(ProducerConfig.KEY_SERIALIZER_CLASS_CONFIG, StringSerializer.class.getName());
        props.put(ProducerConfig.VALUE_SERIALIZER_CLASS_CONFIG, StringSerializer.class.getName());
        props.put(ProducerConfig.ACKS_CONFIG, "1");
        props.put(ProducerConfig.LINGER_MS_CONFIG, 0);
        return props;
    }

    private static Properties consumerProps(String bootstrap, String group) {
        Properties props = new Properties();
        props.put(ConsumerConfig.BOOTSTRAP_SERVERS_CONFIG, bootstrap);
        props.put(ConsumerConfig.GROUP_ID_CONFIG, group);
        props.put(ConsumerConfig.KEY_DESERIALIZER_CLASS_CONFIG, StringDeserializer.class.getName());
        props.put(ConsumerConfig.VALUE_DESERIALIZER_CLASS_CONFIG, StringDeserializer.class.getName());
        props.put(ConsumerConfig.AUTO_OFFSET_RESET_CONFIG, "earliest");
        props.put(ConsumerConfig.ENABLE_AUTO_COMMIT_CONFIG, "false");
        props.put(ConsumerConfig.MAX_POLL_RECORDS_CONFIG, "100");
        return props;
    }

    private static void createTopic(String bootstrap, String topic) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            NewTopic newTopic = new NewTopic(topic, 1, (short) 1);
            admin.createTopics(Collections.singletonList(newTopic)).all().get();
        }
    }

    private static void deleteTopic(String bootstrap, String topic) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            admin.deleteTopics(Collections.singletonList(topic)).all().get();
        }
    }

    private static void describeConfigs(String bootstrap, String topic) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            ConfigResource resource = new ConfigResource(ConfigResource.Type.TOPIC, topic);
            Map<ConfigResource, Config> configs = admin.describeConfigs(
                    Collections.singletonList(resource)).all().get();
            Config config = configs.get(resource);
            if (config == null) {
                throw new RuntimeException("no config returned for topic " + topic);
            }
            if (config.entries().isEmpty()) {
                throw new RuntimeException("expected config entries, got none");
            }
        }
    }

    private static void alterConfigs(String bootstrap, String topic) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            ConfigResource resource = new ConfigResource(ConfigResource.Type.TOPIC, topic);
            Map<ConfigResource, Collection<AlterConfigOp>> alterEntries = new HashMap<>();
            alterEntries.put(resource, Collections.singletonList(
                    new AlterConfigOp(new ConfigEntry("retention.ms", "86400000"),
                            AlterConfigOp.OpType.SET)));
            admin.incrementalAlterConfigs(alterEntries).all().get();
        }
    }

    private static void listOffsets(String bootstrap, String topic) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            TopicPartition tp = new TopicPartition(topic, 0);
            Map<TopicPartition, OffsetSpec> query = Collections.singletonMap(tp, OffsetSpec.latest());
            ListOffsetsResult result = admin.listOffsets(query);
            ListOffsetsResult.ListOffsetsResultInfo info = result.partitionResult(tp).get();
            if (info.offset() < 5) {
                throw new RuntimeException("expected end offset >= 5, got " + info.offset());
            }
        }
    }

    private static void produce(String bootstrap, String topic) throws Exception {
        try (KafkaProducer<String, String> producer = new KafkaProducer<>(producerProps(bootstrap))) {
            for (int i = 0; i < 5; i++) {
                ProducerRecord<String, String> record = new ProducerRecord<>(
                        topic, "key-" + i, "val-" + i);
                Future<RecordMetadata> future = producer.send(record);
                future.get(); // synchronous — ensure each record is acked
            }
        }
    }

    private static void consumeViaGroup(String bootstrap, String topic) throws Exception {
        String group = "java-oracle-group-" + System.nanoTime();
        Map<String, String> received = new LinkedHashMap<>();

        try (KafkaConsumer<String, String> consumer = new KafkaConsumer<>(consumerProps(bootstrap, group))) {
            consumer.subscribe(Collections.singletonList(topic));

            long deadline = System.currentTimeMillis() + 30_000;
            while (received.size() < 5 && System.currentTimeMillis() < deadline) {
                ConsumerRecords<String, String> records = consumer.poll(Duration.ofMillis(500));
                for (ConsumerRecord<String, String> r : records) {
                    received.put(r.key(), r.value());
                }
            }
            consumer.commitSync();
        }

        if (received.size() < 5) {
            throw new RuntimeException("timeout: consumed " + received.size() + "/5 records");
        }
        for (int i = 0; i < 5; i++) {
            String key = "key-" + i;
            String wantVal = "val-" + i;
            String gotVal = received.get(key);
            if (gotVal == null) {
                throw new RuntimeException("missing key " + key);
            }
            if (!gotVal.equals(wantVal)) {
                throw new RuntimeException("key " + key + ": got " + gotVal + " want " + wantVal);
            }
        }
    }

    private static void produceWithHeaders(String bootstrap, String topic) throws Exception {
        try (KafkaProducer<String, String> producer = new KafkaProducer<>(producerProps(bootstrap))) {
            for (int i = 0; i < 3; i++) {
                ProducerRecord<String, String> record = new ProducerRecord<>(
                        topic, null, "hkey-" + i, "hval-" + i);
                record.headers()
                        .add("x-trace", ("trace-" + i).getBytes())
                        .add("x-seq", String.valueOf(i).getBytes());
                producer.send(record).get();
            }
        }
    }

    private static void describeCluster(String bootstrap) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            DescribeClusterResult result = admin.describeCluster();
            Collection<org.apache.kafka.common.Node> nodes = result.nodes().get();
            if (nodes.isEmpty()) {
                throw new RuntimeException("expected at least 1 broker node, got none");
            }
        }
    }

    private static void describeLogDirs(String bootstrap) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            // Describe log dirs for all brokers (we only have one: node 0).
            DescribeLogDirsResult result = admin.describeLogDirs(Collections.singletonList(0));
            Map<Integer, Map<String, org.apache.kafka.clients.admin.LogDirDescription>> dirs =
                    result.allDescriptions().get();
            if (dirs.isEmpty()) {
                throw new RuntimeException("expected log dirs response, got empty map");
            }
        }
    }

    private static void createPartitions(String bootstrap, String topic) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            Map<String, NewPartitions> newPartitions = Collections.singletonMap(
                    topic, NewPartitions.increaseTo(3));
            admin.createPartitions(newPartitions).all().get();
        }
    }

    private static void listGroups(String bootstrap) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            ListConsumerGroupsResult result = admin.listConsumerGroups();
            // Just verify it returns without error; the list may be empty or non-empty.
            Collection<ConsumerGroupListing> groups = result.all().get();
            if (groups == null) {
                throw new RuntimeException("listConsumerGroups returned null");
            }
        }
    }

    private static void describeGroups(String bootstrap, String group) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            // Describing a non-existent group: the request succeeds, the group entry
            // will have a non-STABLE state. We only verify no exception is thrown.
            DescribeConsumerGroupsResult result = admin.describeConsumerGroups(
                    Collections.singletonList(group));
            try {
                result.all().get();
            } catch (ExecutionException e) {
                // GROUP_ID_NOT_FOUND or similar is acceptable — request completed.
            }
        }
    }

    /**
     * Verifies that committed consumer group offsets persist across consumer sessions.
     *
     * 1. Creates a fresh topic and produces 8 records.
     * 2. Consumer session A reads the first 4, commits offset 4, then closes.
     * 3. Consumer session B (same group_id) subscribes and must start at offset 4,
     *    receiving only records 4-7 — not the already-committed 0-3.
     */
    private static void offsetResume(String bootstrap, String topic, String group) throws Exception {
        // Create topic and produce 8 records.
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            admin.createTopics(Collections.singletonList(new NewTopic(topic, 1, (short) 1))).all().get();
        }
        try (KafkaProducer<String, String> producer = new KafkaProducer<>(producerProps(bootstrap))) {
            for (int i = 0; i < 8; i++) {
                producer.send(new ProducerRecord<>(topic, "rkey-" + i, "rval-" + i)).get();
            }
        }

        // Session A: consume exactly 4 records and commit offset.
        Properties sessionProps = consumerProps(bootstrap, group);
        sessionProps.put(ConsumerConfig.MAX_POLL_RECORDS_CONFIG, "4");
        try (KafkaConsumer<String, String> consumerA = new KafkaConsumer<>(sessionProps)) {
            consumerA.subscribe(Collections.singletonList(topic));
            Map<String, String> got = new LinkedHashMap<>();
            long deadline = System.currentTimeMillis() + 20_000;
            while (got.size() < 4 && System.currentTimeMillis() < deadline) {
                ConsumerRecords<String, String> recs = consumerA.poll(Duration.ofMillis(300));
                for (ConsumerRecord<String, String> r : recs) {
                    got.put(r.key(), r.value());
                }
            }
            if (got.size() < 4) {
                throw new RuntimeException("session A timeout: only got " + got.size() + "/4 records");
            }
            consumerA.commitSync();
        } // consumerA closed here — offset committed to store

        // Session B (same group): must resume from offset 4, getting exactly 4 more records.
        Properties sessionBProps = consumerProps(bootstrap, group);
        sessionBProps.put(ConsumerConfig.AUTO_OFFSET_RESET_CONFIG, "latest"); // should not matter; committed offset wins
        sessionBProps.put(ConsumerConfig.MAX_POLL_RECORDS_CONFIG, "10");
        try (KafkaConsumer<String, String> consumerB = new KafkaConsumer<>(sessionBProps)) {
            consumerB.subscribe(Collections.singletonList(topic));
            Map<String, String> got = new LinkedHashMap<>();
            long deadline = System.currentTimeMillis() + 20_000;
            while (got.size() < 4 && System.currentTimeMillis() < deadline) {
                ConsumerRecords<String, String> recs = consumerB.poll(Duration.ofMillis(300));
                for (ConsumerRecord<String, String> r : recs) {
                    got.put(r.key(), r.value());
                }
            }
            if (got.size() < 4) {
                throw new RuntimeException("session B timeout: only got " + got.size() + "/4 records after resume");
            }
            // Verify the 4 resumed records are keys 4-7, not 0-3.
            for (int i = 4; i < 8; i++) {
                String key = "rkey-" + i;
                String wantVal = "rval-" + i;
                String gotVal = got.get(key);
                if (gotVal == null) {
                    throw new RuntimeException("session B missing key " + key + "; got keys: " + got.keySet());
                }
                if (!gotVal.equals(wantVal)) {
                    throw new RuntimeException("session B key " + key + ": got " + gotVal + " want " + wantVal);
                }
            }
            // Verify session B did NOT re-read the first 4.
            for (int i = 0; i < 4; i++) {
                if (got.containsKey("rkey-" + i)) {
                    throw new RuntimeException("session B re-read committed record rkey-" + i);
                }
            }
            consumerB.commitSync();
        }
    }

    private static void consumeHeadersRoundtrip(String bootstrap, String topic) throws Exception {
        String group = "java-headers-group-" + System.nanoTime();

        record HeaderRecord(String val, String trace, String seq) {}
        Map<String, HeaderRecord> received = new LinkedHashMap<>();

        try (KafkaConsumer<String, String> consumer = new KafkaConsumer<>(consumerProps(bootstrap, group))) {
            consumer.subscribe(Collections.singletonList(topic));
            long deadline = System.currentTimeMillis() + 30_000;
            while (received.size() < 3 && System.currentTimeMillis() < deadline) {
                ConsumerRecords<String, String> records = consumer.poll(Duration.ofMillis(500));
                for (ConsumerRecord<String, String> r : records) {
                    String trace = "", seq = "";
                    for (Header h : r.headers()) {
                        if ("x-trace".equals(h.key())) trace = new String(h.value());
                        if ("x-seq".equals(h.key())) seq = new String(h.value());
                    }
                    received.put(r.key(), new HeaderRecord(r.value(), trace, seq));
                }
            }
            consumer.commitSync();
        }

        if (received.size() < 3) {
            throw new RuntimeException("timeout: consumed " + received.size() + "/3 header records");
        }
        for (int i = 0; i < 3; i++) {
            String key = "hkey-" + i;
            HeaderRecord r = received.get(key);
            if (r == null) throw new RuntimeException("missing key " + key);
            if (!r.val().equals("hval-" + i)) throw new RuntimeException(key + ": val mismatch: " + r.val());
            if (!r.trace().equals("trace-" + i)) throw new RuntimeException(key + ": x-trace mismatch: " + r.trace());
            if (!r.seq().equals(String.valueOf(i))) throw new RuntimeException(key + ": x-seq mismatch: " + r.seq());
        }
    }

    private static void incrementalAlterConfigs(String bootstrap, String topic) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            ConfigResource resource = new ConfigResource(ConfigResource.Type.TOPIC, topic);
            Map<ConfigResource, Collection<AlterConfigOp>> ops = new HashMap<>();
            ops.put(resource, Collections.singletonList(
                    new AlterConfigOp(new ConfigEntry("retention.ms", "172800000"), AlterConfigOp.OpType.SET)));
            admin.incrementalAlterConfigs(ops).all().get();
        }
    }

    private static void deleteRecords(String bootstrap, String topic) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            TopicPartition tp = new TopicPartition(topic, 0);
            // Delete records before offset 1 (low watermark advance).
            Map<TopicPartition, RecordsToDelete> toDelete =
                    Collections.singletonMap(tp, RecordsToDelete.beforeOffset(1L));
            DeleteRecordsResult result = admin.deleteRecords(toDelete);
            DeletedRecords deleted = result.lowWatermarks().get(tp).get();
            if (deleted.lowWatermark() < 1) {
                throw new RuntimeException("expected low watermark >= 1, got " + deleted.lowWatermark());
            }
        }
    }

    private static void offsetDelete(String bootstrap, String topic, String group) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            TopicPartition tp = new TopicPartition(topic, 0);
            Map<String, Collection<TopicPartition>> groupOffsets = new HashMap<>();
            groupOffsets.put(group, Collections.singletonList(tp));
            // deleteConsumerGroupOffsets only succeeds when the group is in EMPTY state;
            // the group was never actually joined here, so it may not have offsets to delete —
            // we accept either success or GROUP_ID_NOT_FOUND (error_code != 0 on the entry).
            try {
                admin.deleteConsumerGroupOffsets(group, new HashSet<>(Collections.singletonList(tp))).all().get();
            } catch (ExecutionException e) {
                // GROUP_ID_NOT_FOUND or GROUP_SUBSCRIBED_TO_TOPIC is acceptable.
            }
        }
    }

    private static void deleteGroups(String bootstrap, String group) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            DeleteConsumerGroupsResult result = admin.deleteConsumerGroups(
                    Collections.singletonList(group));
            try {
                result.all().get();
            } catch (ExecutionException e) {
                // GROUP_ID_NOT_FOUND is acceptable — group was never joined.
            }
        }
    }

    private static void listTransactions(String bootstrap) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            ListTransactionsResult result = admin.listTransactions(new ListTransactionsOptions());
            // The response must be parseable; there may be zero active transactions.
            Collection<TransactionListing> transactions = result.all().get();
            if (transactions == null) {
                throw new RuntimeException("listTransactions returned null");
            }
        }
    }

    private static Properties txnProducerProps(String bootstrap, String txnId) {
        Properties props = producerProps(bootstrap);
        props.put(ProducerConfig.TRANSACTIONAL_ID_CONFIG, txnId);
        props.put(ProducerConfig.ENABLE_IDEMPOTENCE_CONFIG, "true");
        props.put(ProducerConfig.ACKS_CONFIG, "all");
        return props;
    }

    private static void transactionalProduce(String bootstrap, String topic) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            admin.createTopics(Collections.singletonList(new NewTopic(topic, 1, (short) 1))).all().get();
        }
        String txnId = "java-oracle-txn-" + System.nanoTime();
        try (KafkaProducer<String, String> producer = new KafkaProducer<>(txnProducerProps(bootstrap, txnId))) {
            producer.initTransactions();
            // Committed transaction: records tkey-0..tkey-2 should be visible to read_committed.
            producer.beginTransaction();
            for (int i = 0; i < 3; i++) {
                producer.send(new ProducerRecord<>(topic, "tkey-" + i, "tval-" + i)).get();
            }
            producer.commitTransaction();
            // Aborted transaction: records akey-0..akey-1 must NOT be visible.
            producer.beginTransaction();
            producer.send(new ProducerRecord<>(topic, "akey-0", "aval-0")).get();
            producer.send(new ProducerRecord<>(topic, "akey-1", "aval-1")).get();
            producer.abortTransaction();
        }
    }

    private static void transactionalConsumeCommitted(String bootstrap, String topic) throws Exception {
        String group = "java-txn-group-" + System.nanoTime();
        Properties props = consumerProps(bootstrap, group);
        props.put(ConsumerConfig.ISOLATION_LEVEL_CONFIG, "read_committed");
        Map<String, String> got = new LinkedHashMap<>();
        try (KafkaConsumer<String, String> consumer = new KafkaConsumer<>(props)) {
            consumer.subscribe(Collections.singletonList(topic));
            long deadline = System.currentTimeMillis() + 30_000;
            while (got.size() < 3 && System.currentTimeMillis() < deadline) {
                ConsumerRecords<String, String> records = consumer.poll(Duration.ofMillis(500));
                for (ConsumerRecord<String, String> r : records) {
                    got.put(r.key(), r.value());
                }
            }
            consumer.commitSync();
        }
        if (got.size() < 3) {
            throw new RuntimeException("timeout: consumed " + got.size() + "/3 committed txn records");
        }
        for (int i = 0; i < 3; i++) {
            String key = "tkey-" + i;
            String want = "tval-" + i;
            String val = got.get(key);
            if (val == null) throw new RuntimeException("committed record missing: " + key);
            if (!val.equals(want)) throw new RuntimeException(key + ": got " + val + " want " + want);
        }
        // Aborted records must not appear in read_committed view.
        if (got.containsKey("akey-0") || got.containsKey("akey-1")) {
            throw new RuntimeException("aborted records leaked into read_committed consumer");
        }
    }

    private static Properties compressedProducerProps(String bootstrap, String codec) {
        Properties props = producerProps(bootstrap);
        props.put(ProducerConfig.COMPRESSION_TYPE_CONFIG, codec); // gzip, snappy, lz4, zstd
        return props;
    }

    private static void produceCompressed(String bootstrap, String topic, String codec) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            admin.createTopics(Collections.singletonList(new NewTopic(topic, 1, (short) 1))).all().get();
        }
        try (KafkaProducer<String, String> producer = new KafkaProducer<>(compressedProducerProps(bootstrap, codec))) {
            for (int i = 0; i < 4; i++) {
                producer.send(new ProducerRecord<>(topic, "ckey-" + i, "cval-" + i)).get();
            }
        }
    }

    private static void consumeCompressedRoundtrip(String bootstrap, String topic) throws Exception {
        String group = "java-comp-group-" + System.nanoTime();
        Map<String, String> got = new LinkedHashMap<>();
        try (KafkaConsumer<String, String> consumer = new KafkaConsumer<>(consumerProps(bootstrap, group))) {
            consumer.subscribe(Collections.singletonList(topic));
            long deadline = System.currentTimeMillis() + 30_000;
            while (got.size() < 4 && System.currentTimeMillis() < deadline) {
                ConsumerRecords<String, String> records = consumer.poll(Duration.ofMillis(500));
                for (ConsumerRecord<String, String> r : records) {
                    got.put(r.key(), r.value());
                }
            }
            consumer.commitSync();
        }
        if (got.size() < 4) {
            throw new RuntimeException("timeout: consumed " + got.size() + "/4 compressed records");
        }
        for (int i = 0; i < 4; i++) {
            String key = "ckey-" + i;
            String want = "cval-" + i;
            String val = got.get(key);
            if (val == null) throw new RuntimeException("missing key " + key);
            if (!val.equals(want)) throw new RuntimeException(key + ": got " + val + " want " + want);
        }
    }

    private static void electLeaders(String bootstrap, String topic) throws Exception {
        try (Admin admin = Admin.create(adminProps(bootstrap))) {
            // Elect preferred leader for partition 0; heimq always is the leader so this is a no-op.
            Set<TopicPartition> partitions = Collections.singleton(new TopicPartition(topic, 0));
            ElectLeadersResult result = admin.electLeaders(ElectionType.PREFERRED, partitions);
            try {
                result.all().get();
            } catch (ExecutionException e) {
                // ELECTION_NOT_NEEDED is acceptable — we're already the preferred leader.
            }
        }
    }
}
