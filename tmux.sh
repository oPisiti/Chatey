#!/bin/bash

# Create a new tmux session named 'Chatey' in detached mode
tmux new-session -d -s Chatey

# Create windows and run the ls command in each
tmux new-window -t Chatey:0 -n 'Nvim'
tmux send-keys -t Chatey:0 'nvim .' Enter

tmux new-window -t Chatey:1 -n 'Server'
tmux send-keys -t Chatey:1 'cargo run -p server' Enter

tmux new-window -t Chatey:2 -n 'Client 1'
tmux send-keys -t Chatey:2 'cargo run -p client' Enter

tmux new-window -t Chatey:3 -n 'Client 2'
tmux send-keys -t Chatey:3 'cargo run -p client' Enter

# Select the first window
tmux select-window -t Chatey:0

# Attach to the session
tmux attach -t Chatey

