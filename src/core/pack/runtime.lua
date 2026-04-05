local __rbx, __lua, __env, __start
do
    local instances = {}
    local modules = {}
    local parentQueue = {}

    local function runModule(object)
        local module = modules[object]
        return module.callback()
    end

    local function requireModule(object)
        local module = modules[object]
        if module.loaded then
            return module.result
        else
            module.result = runModule(object)
            module.loaded = true
            return module.result
        end
    end

    function __rbx(id, parentId, name, className)
        local rbx = Instance.new(className)
        rbx.Name = name
        instances[id] = rbx
        if parentId ~= 0 then
            table.insert(parentQueue, { rbx, parentId })
        end
        return rbx
    end

    function __lua(id, parentId, name, className, callback)
        local rbx = __rbx(id, parentId, name, className)
        modules[rbx] = {
            callback = callback,
            result = nil,
            loaded = false,
            globals = {
                script = rbx,
                require = function(object)
                    if modules[object] then
                        return requireModule(object)
                    else
                        return require(object)
                    end
                end,
            },
        }
    end

    function __env(id)
        return modules[instances[id]].globals
    end

    function __start()
        for _, pair in parentQueue do
            pair[1].Parent = instances[pair[2]]
        end
        for rbx, module in modules do
            if rbx.ClassName == "LocalScript" and not rbx.Disabled then
                task.spawn(module.callback)
            end
        end
    end
end