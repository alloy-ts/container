import { Container, type ContainerOpts } from "@alloy-ts/container";

const opts: ContainerOpts = {
  id: "example",
  rootfs: {
    kind: "directory",
    path: "/var/lib/container-native/rootfs",
  },
  kernelPath: "/var/lib/container-native/vmlinux",
  vcpus: 2,
  memoryMb: 512,
  args: ["/bin/sh", "-c", "echo container started; sleep 30"],
  env: ["HOME=/root", "PATH=/usr/local/bin:/usr/bin:/bin"],
  workingDir: "/",
  network: { mode: "dedicated" },
};

const container = await Container.create(opts);

try {
  console.log(`started ${await container.id()} at ${await container.ipAddr()}`);

  const primary = await container.start();
  const command = await container.exec({
    args: ["/bin/uname", "-a"],
  });

  const output = await command.readStdout(64 * 1024);
  if (output) process.stdout.write(output);

  const commandStatus = await command.wait();
  console.log(`exec exited with ${commandStatus.code}`);

  await primary.kill(15);
  const status = await primary.wait();
  console.log(`container exited with ${status.code}`);
} finally {
  await container.delete();
}
