FROM python:3.12-slim
WORKDIR /bench
RUN pip install --no-cache-dir confluent-kafka==2.6.1
COPY bench.py .
ENTRYPOINT ["python", "bench.py"]
