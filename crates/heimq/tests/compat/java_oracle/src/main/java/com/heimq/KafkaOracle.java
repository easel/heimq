package com.heimq;

import org.apache.kafka.clients.admin.AdminClient;
import org.apache.kafka.clients.admin.NewTopic;
import org.apache.kafka.clients.consumer.ConsumerConfig;
import org.apache.kafka.clients.consumer.ConsumerRecord;
import org.apache.kafka.clients.consumer.ConsumerRecords;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.clients.producer.KafkaProducer;
import org.apache.kafka.clients.producer.ProducerConfig;
import org.apache.kafka.clients.producer.ProducerRecord;
import org.apache.kafka.clients.producer.RecordMetadata;
import org.apache.kafka.common.header.Header;
import org.apache.kafka.common.serialization.StringDeserializer;
import org.apache.kafka.common.serialization.StringSerializer;

import java.time.Duration;
import java.util.*;
import java.util.concurrent.Future;

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
        check("produce", () -> produce(bootstrap, topic));
        check("consume-via-group", () -> consumeViaGroup(bootstrap, topic));
        check("produce-with-headers", () -> produceWithHeaders(bootstrap, topic + "-hdrs"));
        check("consume-headers-roundtrip", () -> consumeHeadersRoundtrip(bootstrap, topic + "-hdrs"));
    }

    @FunctionalInterface
    interface Checker {
        void run() throws Exception;
    }

    private static void check(String name, Checker fn) throws Exception {
        fn.run();
        System.out.printf("  ok  %s%n", name);
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
}
