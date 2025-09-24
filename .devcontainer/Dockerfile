FROM mcr.microsoft.com/devcontainers/rust:1@sha256:6dc904cfd55a32deb51907c7101e0e4fc6ecf31bcd139439f0962a0b79332add

USER root
# Install X11 and input libraries for GUI windowing, events, and cursor support
RUN apt-get update && apt-get install -y \
    libx11-6 \
    libxkbcommon-x11-0 \
    libxrandr2 \
    libxi6 \
    libxcb-render-util0 \
    libxcb-xinput0 \
    libxcb-keysyms1 \
    libxcb-cursor0 \
    libxcursor1 \
    libxss1

# Install OpenGL, Mesa, and rendering libraries for graphics acceleration and offscreen rendering
RUN apt-get install -y \
    libgl1-mesa-glx \
    libegl1-mesa \
    libgles2-mesa \
    libdrm2 \
    libgbm1 \
    libgl1-mesa-dri \
    libglu1-mesa-dev \
    mesa-utils \
    libosmesa6

# Install audio library for sound support
RUN apt-get install -y \
    libasound2

# Clean up apt cache to reduce image size
RUN rm -rf /var/lib/apt/lists/*

USER vscode