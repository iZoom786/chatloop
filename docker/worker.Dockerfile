# ChatLoop Worker Image
#
# This image runs a single ChatLoop inference worker.
# It loads a model partition and processes forward passes.

FROM chatloop-base:latest

# Labels
LABEL maintainer="ChatLoop Team"
LABEL description="ChatLoop Inference Worker"

# Copy configuration templates
COPY configs/worker-config.yaml /home/chatloop/configs/

# Environment variables
ENV CHATLOOP_MODE=worker
ENV CHATLOOP_CONFIG=/home/chatloop/configs/worker-config.yaml
ENV CHATLOOP_BIND_ADDRESS=0.0.0.0
ENV CHATLOOP_PORT=50051

# Expose gRPC port
EXPOSE 50051

# Health check endpoint (if we add HTTP health checks)
EXPOSE 8080

# Metrics endpoint
EXPOSE 9091

# Volumes for model weights and logs
VOLUME ["/home/chatloop/models", "/home/chatloop/logs", "/home/chatloop/configs"]

# Run the worker
CMD ["chatloop-worker"]
