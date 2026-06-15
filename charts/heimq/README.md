# heimq Helm chart

Install a single heimq broker:

```bash
helm install heimq charts/heimq \
  --set image.repository=ghcr.io/easel/heimq \
  --set image.tag=latest
```

Common values:

| Value | Default | Description |
| --- | --- | --- |
| `image.repository` | `ghcr.io/easel/heimq` | Image repository |
| `image.tag` | chart `appVersion` | Image tag |
| `service.type` | `ClusterIP` | Kubernetes Service type |
| `service.port` | `9092` | Kafka listener port |
| `heimq.memoryOnly` | `true` | Run without persistence |
| `heimq.retentionMs` | `604800000` | Retention window in milliseconds |
| `heimq.maxMemoryBytes` | `0` | In-memory byte cap, where `0` is unlimited |
| `heimq.autoCreateTopics` | `true` | Enable topic auto-creation |
| `heimq.advertisedHost` | empty | Optional advertised host for Kafka clients |
