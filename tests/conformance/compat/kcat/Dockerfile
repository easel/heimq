FROM edenhill/kcat:1.7.1
USER root
COPY wait-and-run.sh /usr/local/bin/wait-and-run.sh
COPY kcat/oracle.sh /usr/local/bin/oracle.sh
ENTRYPOINT ["/usr/local/bin/wait-and-run.sh"]
