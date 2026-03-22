# oxidros-cli

ROS2 command-line tool powered by Zenoh — no ROS2 installation required.

## Overview

`oxidros-cli` provides a `ros2` command-line interface that operates entirely over the
Zenoh middleware. It can introspect and interact with a running ROS2 graph without
needing a local ROS2 installation.

## Features

- **Node introspection** — List and inspect active ROS2 nodes
- **Topic tools** — List, echo, pub, and inspect topics
- **Service tools** — List, call, and inspect services
- **Parameter management** — Get, set, and list node parameters
- **Bag recording/playback** — Record and play back MCAP bag files

## Usage

```bash
# List active nodes
ros2 node list

# Echo a topic
ros2 topic echo /chatter

# Call a service
ros2 service call /add_two_ints example_interfaces/srv/AddTwoInts "{a: 1, b: 2}"

# Record a bag
ros2 bag record -o my_bag /chatter
```

## Environment Variables

- `ROS_DOMAIN_ID` — Domain ID (default: `0`)
