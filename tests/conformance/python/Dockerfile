FROM python:3.12-slim

WORKDIR /conformance

# confluent-kafka ships manylinux wheels with librdkafka bundled: no compiler,
# no cmake, no libcurl. The C library is a prebuilt artifact, never a build step.
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY conformance ./conformance

ENTRYPOINT ["python", "-m", "conformance.main"]
