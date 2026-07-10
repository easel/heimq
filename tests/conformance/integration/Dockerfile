FROM python:3.12-slim
WORKDIR /integration

# confluent-kafka: prebuilt manylinux wheel, librdkafka bundled -- no compiler.
# kafka-python-ng: pure Python, and the only client here that can still emit the
# legacy v0/v1 message format the `kafka` crate exercised.
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY conftest.py .
COPY suite ./suite

ENTRYPOINT ["pytest", "-q", "--color=no", "-p", "no:cacheprovider", "suite"]
