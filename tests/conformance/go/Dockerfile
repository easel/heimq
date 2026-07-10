FROM golang:1.25-alpine AS builder

WORKDIR /src

# franz-go is a pure-Go Kafka implementation: no cgo, no librdkafka, no C
# toolchain. It shares no code with the other runner, which is the point.
COPY go.mod go.sum* ./
RUN go mod download

COPY *.go ./
RUN CGO_ENABLED=0 go build -o /conformance-go .

FROM alpine:3.20
COPY --from=builder /conformance-go /usr/local/bin/conformance-go
ENTRYPOINT ["/usr/local/bin/conformance-go"]
