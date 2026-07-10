FROM python:3.12-slim
WORKDIR /e2e
RUN pip install --no-cache-dir confluent-kafka==2.6.1
COPY e2e.py .
ENTRYPOINT ["python", "e2e.py"]
