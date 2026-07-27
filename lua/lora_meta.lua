-- lora_meta.lua
-- DO NOT require this at runtime. Only for LuaLS / IDE.
--- @meta

--- @class Lora
Lora = {
    --- @class Vertex
    --- @field position number[]
    --- @field uv number[]
    --- @field color number[]
    Vertex = {},

    --- @class Point
    --- @field x number
    --- @field y number
    Point = {},

    --- @class Color
    --- @field r number
    --- @field g number
    --- @field b number
    --- @field a number
    Color = {},

    --- @class Shape
    Shape = {
        --- @param self Shape
        --- @param text string
        --- @return nil
        test = function(self, text) end,
    },

    --- @class Collider
    Collider = {},

    --- @class Border
    Border = {},

    --- @class Spawner
    Spawner = {
        --- @param self Spawner
        --- @param x number
        --- @param y number
        --- @param r number
        --- @return Object
        spawn = function(self, x, y, r) return Lora.Object end,
    },

    --- @class Object
    Object = {
        --- @param self Object
        --- @param x number
        --- @param y number
        --- @return nil
        set_position = function(self, x, y) end,
        --- @param self Object
        --- @param x number
        --- @param y number
        --- @return nil
        set_motion = function(self, x, y) end,
        --- @param self Object
        --- @param r number
        --- @return nil
        set_angle = function(self, r) end,
        --- @param self Object
        --- @return number
        --- @return number
        get_position = function(self) return 0, 0 end,
        --- @param self Object
        --- @return number
        --- @return number
        get_motion = function(self) return 0, 0 end,
        --- @param self Object
        --- @return number
        get_angle = function(self) return 0 end,
        --- @param self Object
        --- @param x number
        --- @param y number
        --- @return nil
        impulse = function(self, x, y) end,
        --- @param self Object
        --- @param x number
        --- @param y number
        --- @return nil
        add_force = function(self, x, y) end,
        --- @param self Object
        --- @param x1 number
        --- @param y1 number
        --- @param x2 number
        --- @param y2 number
        --- @return nil
        add_world_force = function(self, x1, y1, x2, y2) end,
        --- @param self Object
        --- @param r number
        --- @return nil
        add_torque = function(self, r) end,
        --- @param self Object
        --- @return nil
        enable = function(self) end,
        --- @param self Object
        --- @return nil
        disable = function(self) end,
        --- @param self Object
        --- @return nil
        toggle = function(self) end,
    },

    --- @class Sound
    Sound = {
        --- @param self Sound
        --- @param volume number
        --- @param pitch number
        --- @return nil
        play = function(self, volume, pitch) end,
        --- @param self Sound
        --- @param count number
        --- @param volume number
        --- @param pitch number
        loop = function(self, count, volume, pitch) end,
        --- @param self Sound
        --- @return nil
        stop = function(self) end,
    },

    --- @class Font
    Font = {},


    --- @return nil
    load = function() end,
    --- @param key string
    --- @return nil
    keypressed = function(key) end,
    --- @param key string
    --- @return nil
    keyreleased = function(key) end,
    --- @param x number
    --- @param y number
    --- @param button integer
    --- @return nil
    mousepressed = function(x, y, button) end,
    --- @param x number
    --- @param y number
    --- @param button integer
    --- @return nil
    mousereleased = function(x, y, button) end,
    --- @param x number
    --- @param y number
    --- @return nil
    mousemoved = function(x, y) end,
    --- @param x number
    --- @param y number
    --- @return nil
    mousescrolled = function(x, y) end,
    --- @param delta number
    --- @return nil
    update = function(delta) end,
    --- @return nil
    render = function() end,
    --- @return nil
    exit = function() end,

    set = {
        window = {
            -- Thingaling! // TODO: Create descriptions for functions
            --- @param text string
            --- @return nil
            title = function(text) end,
            --- @param x integer
            --- @param y integer
            --- @return nil
            size = function(x, y) end,
            --- @param is boolean
            --- @return nil
            resizable = function(is) end,
        },
        physics = {
            --- @param x number
            --- @param y number
            --- @return nil
            gravity = function(x, y) end,
            --- @param hz number
            --- @return nil
            hertz = function(hz) end,
        },
        camera = {
            --- @param x number
            --- @param y number
            --- @return nil
            position = function(x, y) end,
        }
    },

    get = {
        window = {
            --- @return number
            --- @return number
            size = function() return 0, 0 end,
        },
        key = {
            --- @param key string
            --- @return boolean
            state = function(key) return true end,
        },
        mouse = {
            --- @return number[]
            position = function() return {0, 0} end,
        },
        camera = {
            --- @return number[]
            position = function() return {0, 0} end,
        }
    },

    new = {
        --- @param type "rectangle"|"triangle"
        --- @param w number
        --- @param h number
        --- @param color number[]
        --- @return Shape
        shape = function(type, w, h, color) return Lora.Shape end,
        --- @param vertices [number[]]
        --- @param indices integer[] | nil
        --- @return Shape
        mesh = function(vertices, indices) return Lora.Shape end,
        --- @param image string
        --- @param scale number
        --- @return Shape
        image = function(image, scale) return Lora.Shape end,
        --- @param points Point[]
        --- @return Border
        border = function(points) return Lora.Border end,
        --- @param shape Shape
        --- @param collision "static"|"diaxial"|"dynamic"
        --- @return Collider
        collider = function(shape, collision) return Lora.Collider end,
        --- @param shape Shape | nil
        --- @param collider Collider | nil
        --- @return Spawner
        spawner = function(shape, collider) return Lora.Spawner end,
        --- @param sound string
        --- @return Sound
        sound = function(sound) return Lora.Sound end,
        --- @param font string
        --- @return Font
        font = function(font) return Lora.Font end,
    },

    draw = {
        --- @param x1 number
        --- @param y1 number
        --- @param x2 number
        --- @param y2 number
        --- @param radius number
        --- @param color number[]
        --- @return nil
        line = function(x1, y1, x2, y2, radius, color) end,
        --- @param x number
        --- @param y number
        --- @param radius number
        --- @param color number[]
        --- @return nil
        circle = function(x, y, radius, color) end,
        --- @param x number
        --- @param y number
        --- @param w number
        --- @param h number
        --- @param r number
        --- @param color number[]
        --- @return nil
        rect = function(x, y, w, h, r, color) end,
        --- @param x number
        --- @param y number
        --- @param text string
        --- @param font Font | nil
        text = function(x, y, text, font) end,
    }
}

--- @diagnostic disable-next-line
lora = Lora