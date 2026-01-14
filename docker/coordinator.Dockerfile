# ChatLoop Coordinator Image
#
# This image runs the ChatLoop coordinator for routing requests.

FROM chatloop-base:latest

# Labels
LABEL maintainer="ChatLoop Team"
LABEL description="ChatLoop Request Coordinator"

# Copy configuration templates
COPY configs/coordinator-config.yaml /home/chatloop/configs/

# Environment variables
ENV CHATLOOP_MODE=coordinator
ENV CHATLOOP_CONFIG=/home/chatloop/configs/coordinator-config.yaml
ENV CHATLOOP_BIND_ADDRESS=0.0.0.0
ENV CHATLOOP_PORT=50050

# Expose gRPC port
EXPOSE 50050

# Health check endpoint
EXPOSE 8080

# Metrics endpoint
EXPOSE 9091

# Volumes for logs and configs
VOLUME ["/home/chatloop/logs", "/home/chatloop/configs"]

# Run the coordinator
CMD ["chatloop-coordinator"]
