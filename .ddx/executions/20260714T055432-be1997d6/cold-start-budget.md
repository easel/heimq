# Cold-start budget evidence

Command:

```bash
source /tmp/heimq-startup-venv-312/bin/activate && ./scripts/startup-roundtrip-budget.sh
```

Validation notes:

- Broker profile: `release`
- Warm-cache policy: one release build before timing, then three fresh broker processes
- Client: `confluent-kafka` 2.6.1 on Python 3.12

Observed results:

- attempt 1: `elapsed_ms=91.303`
- attempt 2: `elapsed_ms=85.827`
- attempt 3: `elapsed_ms=86.325`

Threshold:

- fail at `>= 1000 ms`

Outcome:

- pass
