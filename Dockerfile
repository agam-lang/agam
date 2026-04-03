FROM ubuntu:22.04
RUN apt-get update && apt-get install -y ca-certificates curl
# Skeleton marker for agam binaries
CMD ["/bin/bash"]
