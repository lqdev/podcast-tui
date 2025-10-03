#!/bin/bash

echo "Testing key bindings for Podcast TUI"
echo "Starting application..."

# Start the application in background
cargo run &
APP_PID=$!

# Give it time to start
sleep 2

# Send 'a' key
echo "Sending 'a' key (should trigger AddPodcast)..."
echo -n "a" | socat - /proc/$APP_PID/fd/0 2>/dev/null || echo "Failed to send 'a'"

sleep 1

# Send 'q' key  
echo "Sending 'q' key (should quit)..."
echo -n "q" | socat - /proc/$APP_PID/fd/0 2>/dev/null || echo "Failed to send 'q'"

sleep 1

# Check if process is still running
if kill -0 $APP_PID 2>/dev/null; then
    echo "Application still running, force killing..."
    kill -TERM $APP_PID
    sleep 1
    if kill -0 $APP_PID 2>/dev/null; then
        kill -KILL $APP_PID
    fi
else
    echo "Application exited normally"
fi

echo "Test completed"