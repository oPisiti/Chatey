#!/bin/bash

# Crossterm is buggy and interprets the Enter keystroke as a "j" char when it comes from a script.
# In order for this script to work, you must alter the handle_input_event() in handlers.rs.
# There must be a special case for the char "j", returning a HandlingSignal::End signal, as such:
#   if char == 'j'{
#     return HandlingSignal::End
#   }

# Create a new tmux session named "Chatey" in detached mode
tmux new-session -d -s Chatey-Rec "exec $SHELL" || true

# Create windows
tmux new-window -t Chatey-Rec:0 -n "Server"
tmux send-keys -t Chatey-Rec:0 "cargo run -p server" Enter
sleep 1

# Initialize windows
chars=("" "Shrek" "Fiona" "Queen" "King" "Donkey")
for ((i=0; i<${#chars[@]}; i++)); do
  tmux new-window -t "Chatey-Rec:$(($i + 1))" -n "${chars[$i]}"
  tmux send-keys -t "Chatey-Rec:$(($i + 1))" "cargo run -p client" Enter
  tmux send-keys -t "Chatey-Rec:$(($i + 1))" "${chars[$i]}" Enter
  sleep 1
done


# Print messages
tmux send-keys -t Chatey-Rec:Queen "Harold!" Enter
sleep 1
tmux send-keys -t Chatey-Rec:Fiona "Shrek!" Enter
sleep 1
tmux send-keys -t Chatey-Rec:Shrek "Fiona!" Enter
sleep 1
tmux send-keys -t Chatey-Rec:King "Fiona!" Enter
sleep 1
tmux send-keys -t Chatey-Rec:Fiona "Mom!" Enter
sleep 1
tmux send-keys -t Chatey-Rec:Queen "Harold!" Enter
sleep 1
tmux send-keys -t Chatey-Rec:Donkey "Donkey!" Enter
sleep 1

# Select Shrek
tmux select-window -t Chatey-Rec:Shrek

# Attach to the session
tmux attach -t Chatey-Rec
