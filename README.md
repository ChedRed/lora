# lora


LORA is a Lua framework that uses Rust's wgpu and winit libraries under the hood, meaning it is extremely fast and capable. At the same time, much of the hard programming is abstracted away into simple Lua functions such as ``lora.set.window.size(..)`` or ``lora.new.object(..)``. Additionally, Rust's rapier2d library is used to handle physics, which has much better performance in complicated scenarios than box2d!

Soon enough there will be a [web url](lora.ched.red) that will hold the docs and more information about the project! For now, however, it is not up, and I'm not quite ready to publish this project.

## Functions

Functions with '=' or '-' are implemented, with '-' meaning untested.
```
[=] lora.set.window.title(text: string) -> nil
[=] lora.set.window.size(w: int, h: int) -> nil
[=] lora.set.window.resizable(is: bool) -> nil
[=] lora.set.physics.gravity(x: number, y: number) -> nil
[=] lora.set.physics.hertz(hz: number) -> nil
[=] lora.set.camera.position(x: number, y: number) -> nil

[=] lora.get.window.size() -> table[w: int, h: int]
[=] lora.get.key.state(key: string) -> bool
[=] lora.get.mouse.position() -> table[x: number, y: number]
[=] lora.get.camera.position() -> table[x: number, y: number]

[=] lora.new.shape(type: string("rectangle"|"triangle"), w: number, h: number, color: number[]) -> lora.Shape
[=] lora.new.mesh(vertices: [number[]], indices: int[] | nil) -> lora.Shape
[ ] lora.new.image(image: string, scale: number) -> lora.Shape
[ ] lora.new.border(points: Point[]) -> lora.Border
[=] lora.new.collider(shape: lora.Shape, collision: string("static"|"diaxial"|"dynamic")) -> lora.Collider
[=] lora.new.spawner(shape: lora.Shape | nil, collider: lora.Collider | nil) -> lora.Spawner
[ ] lora.new.sound(sound: String) -> lora.Sound
[ ] lora.new.font(font: String) -> lora.Font

[=] lora.draw.line(x1: number, y1: number, x2: number, y2: number, color: number[]) -> nil
[=] lora.draw.circle(x: number, y: number, radius: number, color: number[]) -> nil
[=] lora.draw.rect(x: number, y: number, w: number, h: number, r: number, color: number[]) -> nil
[ ] lora.draw.text(x: number, y: number, text: String, font: lora.Font | nil) -> nil

[ ] lora.Sound.play(volume: number, pitch: number) -> nil
[ ] lora.Sound.loop(count: number) -> nil
[ ] lora.Sound.stop() -> nil

[=] lora.Spawner.spawn(x, y, r) -> lora.Object

[=] lora.Object.set_position(x: number, y: number) -> nil
[=] lora.Object.set_motion(x: number, y: number) -> nil
[=] lora.Object.set_angle(r: number) -> nil
[=] lora.Object.get_position() -> table[x: number, y: number]
[=] lora.Object.get_motion() -> table[x: number, y: number]
[=] lora.Object.get_angle() -> number
[=] lora.Object.impulse(x: number, y: number) -> nil
[=] lora.Object.add_force(x: number, y: number) -> nil
[=] lora.Object.add_world_force(x1: number, y1: number, x2: number, y2: number) -> nil
[=] lora.Object.add_torque(r: number) -> nil
[=] lora.Object.enable() -> nil
[=] lora.Object.disable() -> nil
[=] lora.Object.toggle() -> nil

lora.Border.enable() -> nil
lora.Border.disable() -> nil
lora.Border.toggle() -> nil
```

```
[=] lora.load() -> nil
[=] lora.keypressed(key) -> nil
[=] lora.keyreleased(key) -> nil
[=] lora.mousepressed(x, y, button) -> nil
[=] lora.mousereleased(x, y, button) -> nil
[=] lora.mousemoved(x, y) -> nil
[=] lora.mousescrolled(x, y) -> nil
[=] lora.update() -> nil
[=] lora.render() -> nil
[=] lora.exit() -> nil
```

```
[ ] Font
[=] Shape
[ ] Sound
[=] Spawner
[ ] Border
[=] Object
[=] Collider
```

```
TODO:
- Enforce at least one physics tick before rendering, unless lora.update is not present
- Add the rest of the functions ([=] and [-] means fully implemented, but [-] is untested/able)
- Add safeguard to prevent pushing static objects, or objects without colliders (or at least warning)

- Refactor
- Handle exceptions for when lora.update and/or lora.render is not present
- Make github workflow work
- Add friction, density, etc. as well as ticks/second and _ lua-changeable
- Change physics ratio
- Custom shaders?
```