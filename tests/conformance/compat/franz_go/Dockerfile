FROM golang:1.25-alpine AS builder
WORKDIR /src
COPY franz_go/go.mod franz_go/go.sum ./
RUN go mod download
COPY franz_go/*.go ./
RUN CGO_ENABLED=0 go build -o /oracle .

FROM alpine:3.20
COPY --from=builder /oracle /usr/local/bin/oracle
COPY wait-and-run.sh /usr/local/bin/wait-and-run.sh
ENTRYPOINT ["/usr/local/bin/wait-and-run.sh"]
