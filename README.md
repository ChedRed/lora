# lori


Atrocious naming aside, this Lua library uses Rust's wgpu and winit libraries under the hood, meaning it is extremely fast and capable. At the same time, much of the hard programming is abstracted away into simple Lua functions such as ``lori.set.window.size(..)`` or ``lori.delete.object(..)``. Additionally, Rust's rapier2d library is used to handle physics, which has much better performance in complicated scenarios than box2d!

This project has just begun, and I'm hoping it would become a viable alternative to LOVE which, although I do love it, uses the slower OpenGL backend and Box2D physics engine.

## Functions

Functions with '=' or '-' are implemented, with '-' meaning untested.
```
[=] lori.set.window.title(text: string) -> nil
[=] lori.set.window.size(w: int, h: int) -> nil
[=] lori.set.window.resizable(is: bool) -> nil
[=] lori.set.physics.gravity(x: number, y: number) -> nil
[=] lori.set.physics.hertz(hz: number) -> nil
[=] lori.set.camera.position(x: number, y: number) -> nil

[=] lori.get.window.size() -> table[w: int, h: int]
[=] lori.get.key.state(key: string) -> bool
[=] lori.get.mouse.position() -> table[x: number, y: number]
[=] lori.get.camera.position() -> table[x: number, y: number]

[=] lori.new.shape(type: string("rectangle"|"triangle"), w: number, h: number, color: number[]) -> lori.Shape
[=] lori.new.mesh(vertices: [number[]], indices: int[] | nil) -> lori.Shape
[ ] lori.new.image(image: string, scale: number) -> lori.Shape
[ ] lori.new.border(points: Point[]) -> lori.Border
[=] lori.new.collider(shape: lori.Shape, collision: string("static"|"diaxial"|"dynamic")) -> lori.Collider
[=] lori.new.spawner(shape: lori.Shape | nil, collider: lori.Collider | nil) -> lori.Spawner
[ ] lori.new.sound(sound: String) -> lori.Sound
[ ] lori.new.font(font: String) -> lori.Font

[=] lori.draw.line(x1: number, y1: number, x2: number, y2: number, color: number[]) -> nil
[=] lori.draw.circle(x: number, y: number, radius: number, color: number[]) -> nil
[=] lori.draw.rect(x: number, y: number, w: number, h: number, r: number, color: number[]) -> nil
[ ] lori.draw.text(x: number, y: number, text: String, font: lori.Font | nil) -> nil

[ ] lori.Sound.play(volume: number, pitch: number) -> nil
[ ] lori.Sound.loop(count: number) -> nil
[ ] lori.Sound.stop() -> nil

[=] lori.Spawner.spawn(x, y, r) -> lori.Object

[=] lori.Object.set_position(x: number, y: number) -> nil
[=] lori.Object.set_motion(x: number, y: number) -> nil
[=] lori.Object.set_angle(r: number) -> nil
[=] lori.Object.get_position() -> table[x: number, y: number]
[=] lori.Object.get_motion() -> table[x: number, y: number]
[=] lori.Object.get_angle() -> number
[=] lori.Object.impulse(x: number, y: number) -> nil
[=] lori.Object.add_force(x: number, y: number) -> nil
[-] lori.Object.add_world_force(x1: number, y1: number, x2: number, y2: number) -> nil
[=] lori.Object.add_torque(r: number) -> nil
[=] lori.Object.enable() -> nil
[=] lori.Object.disable() -> nil
[=] lori.Object.toggle() -> nil

lori.Border.enable() -> nil
lori.Border.disable() -> nil
lori.Border.toggle() -> nil
```

```
[=] lori.load() -> nil
[=] lori.keypressed(key) -> nil
[=] lori.keyreleased(key) -> nil
[=] lori.mousepressed(x, y, button) -> nil
[=] lori.mousereleased(x, y, button) -> nil
[=] lori.mousemoved(x, y) -> nil
[=] lori.mousescrolled(x, y) -> nil
[=] lori.update() -> nil
[=] lori.render() -> nil
[=] lori.exit() -> nil
```

```
[ ] Font
[ ] Point
[=] Shape
[ ] Sound
[=] Spawner
[ ] Border
[=] Object
[ ] Vertex
[=] Collider
```

```
TODO:
- Enforce at least one physics tick before rendering, unless lori.update is not present
- Add the rest of the functions ([=] and [-] means fully implemented, but [-] is untested/able)
- Add safeguard to prevent pushing static objects, or objects without colliders (or at least warning)

- Refactor
- Handle exceptions for when lori.update and/or lori.render is not present
- Make github workflow work
- Add friction, density, etc. as well as ticks/second and _ lua-changeable
- Change physics ratio
```