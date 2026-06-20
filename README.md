# Project Structure

[Unity/](Unity) contains the Unity project which runs on the headset.  
[main.py](main.py) is the text detection inference system.  
[src/](src) contains the frontend user interface and also communicates with both the headset and inference systems.  

# Running

You will need:
 - Unity
 - The full meta quest development environment, including the Meta Horizon Link app
 - [uv](https://docs.astral.sh/uv/) for managing the python environment
 - The [rust compiler](https://rustup.rs/)

Then, run `uv sync` to create the python virtual environment and ensure your shell has entered it.

You can then run the unity project on the headset, and run the frontend with `cargo run --release` (the order does not matter as the socket is stateless). 
