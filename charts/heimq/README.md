# heimq Helm chart

Install a single heimq broker from the source chart:

```bash
helm install heimq charts/heimq \
  --set image.repository=ghcr.io/easel/heimq \
  --set image.tag=0.1.0
```

Install the published OCI chart from GHCR:

```bash
helm install heimq oci://ghcr.io/easel/charts/heimq \
  --version 0.1.0
```

Upgrade an existing install and expose it through a LoadBalancer:

```bash
helm upgrade --install heimq charts/heimq \
  --set image.repository=ghcr.io/easel/heimq \
  --set image.tag=0.1.0 \
  --set service.type=LoadBalancer
```

Override runtime defaults:

```bash
helm upgrade --install heimq charts/heimq \
  --set heimq.memoryOnly=true \
  --set heimq.retentionMs=3600000 \
  --set heimq.maxMemoryBytes=268435456
```

Common values:

| Value | Default | Description |
| --- | --- | --- |
| `image.repository` | `ghcr.io/easel/heimq` | Image repository |
| `image.tag` | chart `appVersion` | Image tag |
| Published chart path | `oci://ghcr.io/easel/charts/heimq` | GHCR OCI chart registry path |
| `service.type` | `ClusterIP` | Kubernetes Service type |
| `service.port` | `9092` | Kafka listener port |
| `heimq.memoryOnly` | `true` | Run without persistence |
| `heimq.retentionMs` | `604800000` | Retention window in milliseconds |
| `heimq.maxMemoryBytes` | `0` | In-memory byte cap, where `0` is unlimited |
| `heimq.autoCreateTopics` | `true` | Enable topic auto-creation |
| `heimq.advertisedHost` | empty | Optional advertised host for Kafka clients |
