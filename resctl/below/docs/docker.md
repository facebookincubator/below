`below` depends on fbthrift which in turn depends on
[folly](https://github.com/facebook/folly),
[wangle](https://github.com/facebook/wangle),
[glog](https://github.com/google/glog), and other tricky-to-build dependencies.
To ease the pain, we provide a Dockerfile as well as prebuilt images on docker
hub.

Note the instructions use `podman` instead of `docker` because at time of
writing, docker doesn't yet have support for cgroup2.

# Prebuilt image

```shell
$ podman run --privileged --cgroupns=host --pid=host -it below/below:latest
```

# Local build

```shell
$ git clone https://github.com/facebookincubator/resctl.git ~/dev/resctl
<...>

$ cd ~/dev/resctl

$ podman build -t below .
<...>

$ podman run --privileged --cgroupns=host --pid=host -it below
```
