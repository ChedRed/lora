local PlayerJumped = false
local PlayertwoJumped = false

function lora.load()
    lora.set.physics.gravity(0, -50)
    lora.set.physics.hertz(200)

    PlayerShape = lora.new.image("resources/larry.png", 0.5)
    PlayerCollider = lora.new.collider(PlayerShape, "diaxial")
    PlayerSpawner = lora.new.spawner(PlayerShape, PlayerCollider)
    PlayerObject = PlayerSpawner:spawn(200, 100, 0)

    PlayertwoShape = lora.new.image("resources/snowmog.png", 0.5)
    PlayertwoCollider = lora.new.collider(PlayertwoShape, "diaxial")
    PlayertwoSpawner = lora.new.spawner(PlayertwoShape, PlayertwoCollider)
    PlayertwoObject = PlayertwoSpawner:spawn(2360, 100, 0)

    MapBorder = lora.new.border({
        { 0,    0 },
        { 2560, 0 },
    })

    MapUpperBorder = lora.new.border({
        { 2560, 0 },
        { 2560, 1600 },
        { 0,    1600 },
        { 0,    0 },
    })
end

function lora.collision(one, two)
    if one == MapBorder:id() or two == MapBorder:id() then
        if one == PlayerObject:id() or two == PlayerObject:id() then
            PlayerJumped = false
        else
            if one == PlayertwoObject:id() or two == PlayertwoObject:id() then
                PlayertwoJumped = false
            end
        end
    end
end

function lora.keypressed(key)
    if key == "w" then
        if not PlayerJumped then
            PlayerJumped = true
            PlayerObject:impulse(0, 84)
        end
    end
    if key == "i" then
        if not PlayertwoJumped then
            PlayertwoJumped = true
            PlayertwoObject:impulse(0, 84)
        end
    end
end

function lora.update(delta)
    if lora.get.key.state("a") then
        PlayerObject:impulse(-1, 0)
    end
    if lora.get.key.state("d") then
        PlayerObject:impulse(1, 0)
    end

    if lora.get.key.state("j") then
        PlayertwoObject:impulse(-1, 0)
    end
    if lora.get.key.state("l") then
        PlayertwoObject:impulse(1, 0)
    end
end