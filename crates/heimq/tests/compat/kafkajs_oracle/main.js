#!/usr/bin/env node
// KafkaJS compatibility oracle for heimq.
//
// KafkaJS is a pure-JavaScript Kafka client that implements the protocol
// independently of librdkafka, franz-go, sarama, and the Java client.
// It provides a 6th independent client perspective for the external oracle suite.
//
// Usage: node main.js <bootstrap-servers>
// Exit 0 on success; non-zero with a FAIL line on any deviation.

const { Kafka, logLevel } = require('kafkajs');

const bootstrap = process.argv[2];
if (!bootstrap) {
  console.error('usage: node main.js <bootstrap-servers>');
  process.exit(1);
}

const ts = Date.now();
const topic = `kafkajs-compat-${ts}`;
const groupId = `kafkajs-group-${ts}`;
const headerTopic = `kafkajs-hdrs-${ts}`;
const headerGroup = `kafkajs-hdrs-group-${ts}`;
const resumeTopic = `kafkajs-resume-${ts}`;
const resumeGroup = `kafkajs-resume-${ts}`;

const kafka = new Kafka({
  clientId: 'kafkajs-oracle',
  brokers: [bootstrap],
  logLevel: logLevel.ERROR,
  connectionTimeout: 10_000,
  requestTimeout: 10_000,
  retry: { initialRetryTime: 100, retries: 5 },
});

async function check(name, fn) {
  try {
    await fn();
    console.log(`  ok  ${name}`);
  } catch (err) {
    console.error(`FAIL  ${name}: ${err.message}`);
    process.exit(1);
  }
}

async function createTopic(admin, t, numPartitions = 1) {
  await admin.createTopics({
    waitForLeaders: false,
    topics: [{ topic: t, numPartitions, replicationFactor: 1 }],
  });
}

// Consume exactly `want` messages from `t` using consumer group `gid`.
// Times out after `timeoutMs` ms.
async function consumeGroup(t, gid, want, timeoutMs = 15_000) {
  const consumer = kafka.consumer({ groupId: gid });
  await consumer.connect();
  await consumer.subscribe({ topic: t, fromBeginning: true });

  const received = [];
  let resolve_, reject_;
  const done = new Promise((res, rej) => { resolve_ = res; reject_ = rej; });

  const timeoutId = setTimeout(() => {
    reject_(new Error(`timed out after receiving ${received.length}/${want} messages`));
  }, timeoutMs);

  await consumer.run({
    eachMessage: async ({ message }) => {
      received.push(message);
      if (received.length >= want) {
        clearTimeout(timeoutId);
        resolve_(received);
      }
    },
  });

  const msgs = await done;
  await consumer.disconnect();
  return msgs;
}

async function run() {
  const admin = kafka.admin();
  await admin.connect();

  // --- create-topic ---
  await check('create-topic', () => createTopic(admin, topic));

  // --- produce ---
  const producer = kafka.producer({ allowAutoTopicCreation: false });
  await producer.connect();

  const messages = Array.from({ length: 5 }, (_, i) => ({
    key: `key-${i}`,
    value: `value-${i}`,
  }));

  await check('produce', async () => {
    await producer.send({ topic, messages });
  });

  // --- consume-via-group ---
  await check('consume-via-group', async () => {
    const msgs = await consumeGroup(topic, groupId, 5);
    if (msgs.length !== 5) throw new Error(`expected 5, got ${msgs.length}`);
    for (let i = 0; i < 5; i++) {
      const val = msgs[i].value.toString();
      if (val !== `value-${i}`) {
        throw new Error(`msg[${i}]: expected "value-${i}", got "${val}"`);
      }
    }
  });

  // --- produce-with-headers ---
  await check('create-header-topic', () => createTopic(admin, headerTopic));

  await check('produce-with-headers', async () => {
    await producer.send({
      topic: headerTopic,
      messages: [{
        key: 'hdr-key',
        value: 'hdr-value',
        headers: {
          'x-trace-id': 'abc123',
          'x-env': 'test',
        },
      }],
    });
  });

  // --- consume-headers-roundtrip ---
  await check('consume-headers-roundtrip', async () => {
    const msgs = await consumeGroup(headerTopic, headerGroup, 1, 10_000);
    const msg = msgs[0];
    const traceId = msg.headers['x-trace-id']?.toString();
    const env = msg.headers['x-env']?.toString();
    if (traceId !== 'abc123') throw new Error(`x-trace-id: expected "abc123", got "${traceId}"`);
    if (env !== 'test') throw new Error(`x-env: expected "test", got "${env}"`);
  });

  // --- fetch-topic-offsets ---
  await check('fetch-topic-offsets', async () => {
    const offsets = await admin.fetchTopicOffsets(topic);
    if (!Array.isArray(offsets) || offsets.length === 0) {
      throw new Error('fetchTopicOffsets returned empty result');
    }
    const p0 = offsets.find(o => o.partition === 0);
    if (!p0) throw new Error('partition 0 not found');
    const high = parseInt(p0.high, 10);
    if (high < 5) throw new Error(`high watermark ${high} < 5`);
  });

  // --- list-groups --- (must run before delete-groups cleans up the group)
  await check('list-groups', async () => {
    const groups = await admin.listGroups();
    if (!groups || !Array.isArray(groups.groups)) {
      throw new Error('listGroups returned unexpected shape');
    }
    // After consume-via-group the group must appear.
    const found = groups.groups.find(g => g.groupId === groupId);
    if (!found) {
      throw new Error(`group "${groupId}" not found in listGroups result`);
    }
  });

  // --- describe-groups ---
  await check('describe-groups', async () => {
    const descriptions = await admin.describeGroups([groupId]);
    if (!descriptions.groups || descriptions.groups.length === 0) {
      throw new Error('describeGroups returned no groups');
    }
    const group = descriptions.groups[0];
    if (group.groupId !== groupId) {
      throw new Error(`expected group "${groupId}", got "${group.groupId}"`);
    }
  });

  // --- delete-groups ---
  await check('delete-groups', async () => {
    const results = await admin.deleteGroups([groupId, headerGroup]);
    for (const r of results) {
      // 0 = no error; 69 = GROUP_ID_NOT_FOUND (group already gone — ok)
      if (r.errorCode !== 0 && r.errorCode !== 69) {
        throw new Error(`deleteGroup ${r.groupId} error code ${r.errorCode}`);
      }
    }
  });

  // --- describe-configs ---
  await check('describe-configs', async () => {
    const configs = await admin.describeConfigs({
      resources: [{ type: admin.constructor.ResourceTypes?.TOPIC ?? 2, name: topic }],
      includeSynonyms: false,
    });
    if (!configs || !Array.isArray(configs.resources) || configs.resources.length === 0) {
      throw new Error('describeConfigs returned no resources');
    }
    if (configs.resources[0].errorCode !== 0) {
      throw new Error(`describeConfigs error code ${configs.resources[0].errorCode}`);
    }
  });

  // --- offset-resume ---
  // Produce 4 more records, consume 2, commit, reconnect, verify only the
  // remaining 2 arrive (group resumes from the committed offset).
  await check('create-resume-topic', () => createTopic(admin, resumeTopic));
  const resumeProducer = kafka.producer({ allowAutoTopicCreation: false });
  await resumeProducer.connect();
  await resumeProducer.send({
    topic: resumeTopic,
    messages: Array.from({ length: 4 }, (_, i) => ({
      key: `rkey-${i}`,
      value: `rval-${i}`,
    })),
  });
  await resumeProducer.disconnect();

  await check('offset-resume', async () => {
    // Session A: consume first 2, commit.
    const consumerA = kafka.consumer({ groupId: resumeGroup });
    await consumerA.connect();
    await consumerA.subscribe({ topic: resumeTopic, fromBeginning: true });

    let countA = 0;
    let resolveA;
    const doneA = new Promise(r => { resolveA = r; });
    await consumerA.run({
      eachMessage: async ({ message, heartbeat }) => {
        countA++;
        if (countA === 2) {
          await consumerA.commitOffsets([{
            topic: resumeTopic,
            partition: 0,
            offset: String(parseInt(message.offset) + 1),
          }]);
          resolveA();
        }
      },
    });
    await doneA;
    await consumerA.disconnect();

    // Session B: should only see the remaining 2 records.
    const received = [];
    const consumerB = kafka.consumer({ groupId: resumeGroup });
    await consumerB.connect();
    await consumerB.subscribe({ topic: resumeTopic, fromBeginning: true });

    let resolveB;
    const doneB = new Promise(r => { resolveB = r; });
    const timeoutB = setTimeout(() => resolveB(), 8_000);

    await consumerB.run({
      eachMessage: async ({ message }) => {
        received.push(message.value.toString());
        if (received.length >= 2) {
          clearTimeout(timeoutB);
          resolveB();
        }
      },
    });
    await doneB;
    await consumerB.disconnect();

    if (received.length !== 2) {
      throw new Error(`expected 2 records after resume, got ${received.length}`);
    }
    if (received[0] !== 'rval-2' || received[1] !== 'rval-3') {
      throw new Error(`unexpected values after resume: ${JSON.stringify(received)}`);
    }
  });

  // --- delete-topics ---
  await check('delete-topics', async () => {
    await admin.deleteTopics({ topics: [topic, headerTopic, resumeTopic] });
  });

  await producer.disconnect();
  await admin.disconnect();
}

run().then(() => {
  console.log('PASS');
}).catch(err => {
  console.error(`FAIL: ${err.message}`);
  process.exit(1);
});
