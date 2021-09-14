return {
  version = "1.5",
  luaversion = "5.1",
  tiledversion = "1.7.2",
  orientation = "orthogonal",
  renderorder = "right-down",
  width = 227,
  height = 18,
  tilewidth = 16,
  tileheight = 16,
  nextlayerid = 22,
  nextobjectid = 83,
  properties = {},
  tilesets = {
    {
      name = "NES - Super Mario Bros - Tileset",
      firstgid = 1,
      tilewidth = 16,
      tileheight = 16,
      spacing = 1,
      margin = 1,
      columns = 48,
      image = "NES - Super Mario Bros - Tileset.png",
      imagewidth = 817,
      imageheight = 137,
      transparentcolor = "#93bbec",
      objectalignment = "unspecified",
      tileoffset = {
        x = 0,
        y = 0
      },
      grid = {
        orientation = "orthogonal",
        width = 16,
        height = 16
      },
      properties = {},
      wangsets = {},
      tilecount = 384,
      tiles = {
        {
          id = 0,
          properties = {
            ["fire_flower"] = 1
          },
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 6.78775,
                y = 7.84834,
                width = 6,
                height = 6,
                rotation = 0,
                visible = true,
                properties = {}
              },
              {
                id = 2,
                name = "",
                type = "",
                shape = "point",
                x = 4.125,
                y = 9.375,
                width = 0,
                height = 0,
                rotation = 0,
                visible = true,
                properties = {}
              },
              {
                id = 3,
                name = "",
                type = "",
                shape = "ellipse",
                x = 13.5,
                y = 2.75,
                width = 4,
                height = 4,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          },
          animation = {
            {
              tileid = 0,
              duration = 400
            },
            {
              tileid = 1,
              duration = 132
            },
            {
              tileid = 2,
              duration = 132
            },
            {
              tileid = 3,
              duration = 132
            }
          }
        },
        {
          id = 48,
          properties = {
            ["star"] = 1
          },
          animation = {
            {
              tileid = 48,
              duration = 50
            },
            {
              tileid = 49,
              duration = 50
            },
            {
              tileid = 50,
              duration = 50
            },
            {
              tileid = 51,
              duration = 50
            }
          }
        },
        {
          id = 53,
          properties = {
            ["touch_to_win"] = true
          },
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "ellipse",
                x = 3.5,
                y = 7.5,
                width = 9,
                height = 9,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 69,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "ellipse",
                x = 3.5,
                y = 7.5,
                width = 9,
                height = 9,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 96,
          properties = {
            ["coin"] = 1
          },
          animation = {
            {
              tileid = 96,
              duration = 400
            },
            {
              tileid = 97,
              duration = 132
            },
            {
              tileid = 98,
              duration = 132
            },
            {
              tileid = 99,
              duration = 132
            }
          }
        },
        {
          id = 101,
          properties = {
            ["touch_to_win"] = true
          },
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 7,
                y = 0,
                width = 2,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 102,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 103,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 117,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 7,
                y = 0,
                width = 2,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 144,
          properties = {
            ["hittable"] = 148
          },
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          },
          animation = {
            {
              tileid = 144,
              duration = 400
            },
            {
              tileid = 145,
              duration = 132
            },
            {
              tileid = 146,
              duration = 132
            },
            {
              tileid = 147,
              duration = 132
            }
          }
        },
        {
          id = 145,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 146,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 147,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 148,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 149,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 150,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 3,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 2,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 151,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 192,
          properties = {
            ["touch_to_win"] = true
          }
        },
        {
          id = 199,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 241,
          properties = {
            ["breakable"] = true
          },
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 242,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 247,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 336,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 338,
          objectGroup = {
            type = "objectgroup",
            draworder = "index",
            id = 2,
            name = "",
            visible = true,
            opacity = 1,
            offsetx = 0,
            offsety = 0,
            parallaxx = 1,
            parallaxy = 1,
            properties = {},
            objects = {
              {
                id = 1,
                name = "",
                type = "",
                shape = "rectangle",
                x = 0,
                y = 0,
                width = 16,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        }
      }
    }
  },
  layers = {
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 227,
      height = 18,
      id = 9,
      name = "Sky",
      visible = true,
      opacity = 1,
      offsetx = 0,
      offsety = 0,
      parallaxx = 1,
      parallaxy = 1,
      properties = {},
      encoding = "base64",
      compression = "zlib",
      data = "eJzt07ENwCAAA0FYMktk/54qE4DEKzpL17v5MczMzOzbCxzxzD23/8NfaBEatAgNWoQGLUKDFqFBi9CgRWjQIjRoERq0CA1ahAYtQoMWoUGL0LDb4gIu1tpc"
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 227,
      height = 18,
      id = 12,
      name = "Background",
      visible = true,
      opacity = 1,
      offsetx = 0,
      offsety = 0,
      parallaxx = 1,
      parallaxy = 1,
      properties = {},
      encoding = "base64",
      compression = "zlib",
      data = "eJzt129OhDAQh+FyGPVE6t3UqOcxRr3LfvLf6pBlsyMhZQoDbfF9ksnuB2jnR2mAEAAAAADE3Es9dPWYaW6veXNm8VRzjpp7n2rsPtbXJHbci9RrV28r9DU0t2VeSx7vLKms2cey1JJjSO7ecxi7j/U1WfO6pOyvKeOWvM7W7KVnWWoNAQDb4f1tm4NnBus3EOBtC+9tnhlKf88GAAAAlnbWnApAPpeyB3fye73AXrz1HzKLreRobSVLbTnG+m2fhReqPJ+NT1IfUs8Tzy/lWs/JUUqGo7lrchMOme7cOpqmtnvL0u+uq/Pm9N/Dezf3p9SX1Hfk2L3Uj6rQxHvfq+Pm0nMPjWfNkZqhf55HlqFe9Lgpa9IfZ+11CXrentQcf6ycoWXt97j/rvRebA7na5a8sXsaAID/7BfYu7KD"
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 227,
      height = 18,
      id = 11,
      name = "Foreground",
      visible = true,
      opacity = 1,
      offsetx = 0,
      offsety = 0,
      parallaxx = 1,
      parallaxy = 1,
      properties = {},
      encoding = "base64",
      compression = "zlib",
      data = "eJztmU1uwjAYBYO4U+/RI3QDvU42aS8E3Aep7bauRCQr9c9zHMdOmJGeCMpnYxuPgkXXAQAAAAAAAABAKY6H/+99mfKxzhABNs2LWOdyTAUXAeKcxDpchBzuVlz3+kdc95+FrbvYF+y7Ve4J8bXLWbdnXPMchkdibN1FgJYZOlwEqM3Q4SJAbaYe7snF6RllPPPYryU5m7wvWJfKWvP09d8L12pfvtpQ/eC5jjH1IKXtiH3GVubk8nBPLpZE8edvDp9CX2od6Lh8Uva2ywO17Vx8HpZ0sfX/+lOeT4o/uFiHkE+h/e3zINY29XeNXR/ysLaLF5OryU38jCVJcQIX20TxybXHQx7E2qZ+f2N9zEP1Oa4wx8Uvk2+TH5PXA3HFpkSfS/a75expLd4yU3v8rQYX113n2uNYIrku/gKFXAfu"
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 227,
      height = 18,
      id = 14,
      name = "ItemsInBlocks",
      visible = false,
      opacity = 1,
      offsetx = 0,
      offsety = 0,
      parallaxx = 1,
      parallaxy = 1,
      properties = {},
      encoding = "base64",
      compression = "zlib",
      data = "eJzt1zEKQCEIANA6qve/REtjfIIGs/8eCIKLioutAQAAAAAAQG0xg7p6dgNFndy9nQMAADdZ/TfxUXtR5p8WG/nr/jQrAAAAADkGajYDzg=="
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 227,
      height = 18,
      id = 19,
      name = "Items",
      visible = true,
      opacity = 1,
      offsetx = 0,
      offsety = 0,
      parallaxx = 1,
      parallaxy = 1,
      properties = {},
      encoding = "base64",
      compression = "zlib",
      data = "eJzt1UENAAAIxDCs4t8EBu5PAm0yDasCAAAAAAAAALZ1CAA+Sk/0RgC+8kUAAACAOwarQgc0"
    },
    {
      type = "objectgroup",
      draworder = "topdown",
      id = 18,
      name = "Unique Tiles",
      visible = true,
      opacity = 1,
      offsetx = 0,
      offsety = 0,
      parallaxx = 1,
      parallaxy = 1,
      properties = {},
      objects = {
        {
          id = 28,
          name = "Timer Coin Block",
          type = "",
          shape = "rectangle",
          x = 1600,
          y = 208,
          width = 16,
          height = 16,
          rotation = 0,
          gid = 242,
          visible = true,
          properties = {
            ["amount"] = 10,
            ["breakable"] = false,
            ["spawns"] = "coin",
            ["timer"] = 5,
            ["turns_into"] = 148
          }
        },
        {
          id = 29,
          name = "1 up hidden block",
          type = "",
          shape = "rectangle",
          x = 1056,
          y = 192,
          width = 16,
          height = 16,
          rotation = 0,
          gid = 145,
          visible = true,
          properties = {
            ["spawns"] = "1-up"
          }
        },
        {
          id = 64,
          name = "Star Block",
          type = "",
          shape = "rectangle",
          x = 1712,
          y = 208,
          width = 16,
          height = 16,
          rotation = 0,
          gid = 242,
          visible = true,
          properties = {
            ["breakable"] = false,
            ["spawns"] = "star",
            ["turns_into"] = 148
          }
        }
      }
    },
    {
      type = "objectgroup",
      draworder = "topdown",
      id = 17,
      name = "Warp Zones",
      visible = true,
      opacity = 1,
      offsetx = 0,
      offsety = 0,
      parallaxx = 1,
      parallaxy = 1,
      properties = {},
      objects = {
        {
          id = 47,
          name = "In 1",
          type = "",
          shape = "point",
          x = 944,
          y = 192,
          width = 0,
          height = 0,
          rotation = 0,
          visible = true,
          properties = {
            ["direction"] = "down",
            ["output"] = { id = 48 }
          }
        },
        {
          id = 48,
          name = "Out 1",
          type = "",
          shape = "point",
          x = 3408,
          y = 64,
          width = 0,
          height = 0,
          rotation = 0,
          visible = true,
          properties = {
            ["direction"] = "down"
          }
        },
        {
          id = 49,
          name = "In 2",
          type = "",
          shape = "point",
          x = 3584,
          y = 240,
          width = 0,
          height = 0,
          rotation = 0,
          visible = true,
          properties = {
            ["direction"] = "right",
            ["output"] = { id = 50 }
          }
        },
        {
          id = 50,
          name = "Out 2",
          type = "",
          shape = "point",
          x = 2736,
          y = 224,
          width = 0,
          height = 0,
          rotation = 0,
          visible = true,
          properties = {
            ["direction"] = "up"
          }
        }
      }
    },
    {
      type = "objectgroup",
      draworder = "topdown",
      id = 20,
      name = "Flag",
      visible = true,
      opacity = 1,
      offsetx = 0,
      offsety = 0,
      parallaxx = 1,
      parallaxy = 1,
      properties = {},
      objects = {
        {
          id = 46,
          name = "Flag Pole",
          type = "",
          shape = "rectangle",
          x = 3207,
          y = 112,
          width = 16,
          height = 16,
          rotation = 0,
          gid = 105,
          visible = true,
          properties = {}
        }
      }
    },
    {
      type = "objectgroup",
      draworder = "topdown",
      id = 21,
      name = "Characters",
      visible = true,
      opacity = 1,
      offsetx = 0,
      offsety = 0,
      parallaxx = 1,
      parallaxy = 1,
      properties = {},
      objects = {
        {
          id = 55,
          name = "Mario",
          type = "",
          shape = "rectangle",
          x = 48,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["player"] = 1
          }
        },
        {
          id = 56,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 352,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 57,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 656,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 58,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 848,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 59,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 874,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 60,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 1328,
          y = 112,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 61,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 1360,
          y = 112,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 62,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 1632,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 63,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 1648,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 65,
          name = "Koopa",
          type = "",
          shape = "rectangle",
          x = 1808,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "g_koopa"
          }
        },
        {
          id = 66,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 1936,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 67,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 1968,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 68,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 2080,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 69,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 2096,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 70,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 2176,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 71,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 2160,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 72,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 2848,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        },
        {
          id = 73,
          name = "Goomba",
          type = "",
          shape = "rectangle",
          x = 2880,
          y = 240,
          width = 16,
          height = 16,
          rotation = 0,
          visible = true,
          properties = {
            ["enemy"] = "goomba"
          }
        }
      }
    }
  }
}
