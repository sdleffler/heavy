return {
  version = "1.5",
  luaversion = "5.1",
  tiledversion = "1.7.2",
  orientation = "orthogonal",
  renderorder = "right-down",
  width = 227,
  height = 34,
  tilewidth = 16,
  tileheight = 16,
  nextlayerid = 22,
  nextobjectid = 78,
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
          id = 16,
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
          id = 48,
          properties = {
            ["star"] = 1
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
                x = 1,
                y = 0,
                width = 14,
                height = 16,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
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
          id = 64,
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
                x = 1,
                y = 0,
                width = 14,
                height = 16,
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
                x = 3,
                y = 2,
                width = 10,
                height = 14,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
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
          id = 112,
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
                x = 3,
                y = 2,
                width = 10,
                height = 14,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          },
          animation = {
            {
              tileid = 112,
              duration = 400
            },
            {
              tileid = 113,
              duration = 132
            },
            {
              tileid = 114,
              duration = 132
            },
            {
              tileid = 115,
              duration = 132
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
          id = 160,
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
          id = 166,
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
                x = 2,
                y = 0,
                width = 14,
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
          id = 196,
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
          id = 197,
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
                y = 1,
                width = 16,
                height = 15,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 198,
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
          id = 212,
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
          id = 213,
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
                y = 1,
                width = 16,
                height = 15,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 214,
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
          id = 244,
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
          id = 245,
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
                height = 15,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 246,
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
          id = 257,
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
          id = 258,
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
          id = 260,
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
          id = 261,
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
                height = 15,
                rotation = 0,
                visible = true,
                properties = {}
              }
            }
          }
        },
        {
          id = 262,
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
        },
        {
          id = 352,
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
      height = 34,
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
      compression = "gzip",
      data = "H4sIAAAAAAAACu3TsQkCABRDQV3SJdy/t3KCCPnEO3h9mjweAMDXW9JPSrX3Syul2vullVLt/dJKqfZ+aaVUe7+0Uqq9X1op1d4vrZRq75dWSrX3Syul2vullVLt/dJKqfZ+aaVUe7+0Uqq9X1op1d4vrcSe1zML+A1fhBt8EW7wRbjBF+EGX4QbfBFu8EW4wRfhBl+EG3wRbvBFuMEX4QZfhBt8EW7wxf/0AYmHvIyYeAAA"
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 227,
      height = 34,
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
      compression = "gzip",
      data = "H4sIAAAAAAAACu3bW07CQBQG4LIYdUXq3tSo6zFE3QtPeEGHWMLYNO20DL3l+5ITICkz5+90UvpAUQAAAE0eQz2V9TzS3LnmHTNLTnPOMefe+2q7juNz0nTca6i3st4H6Ktu7pR5U/LkztJVava2LHPJUWfs3sfQdh3H52TI89Jlf/UZd8rrnJp96lnOtYYALEfuZ9sx5MyQ+gwEuS3hd1vODFP/nQ0AAOd2sToWMJ7rsAc34fX2DHvxPv+Qo1hKjr2lZJlbjrZ+9/fCq6hy3htfQn2EWvf8/lTO9Sk5ppLh4NQ1uSv+Mj1k66ifuV1bKf1uyrpcHd/nsC3n/gz1Feq74dhdqJ+oilVz77vouFPFc9eNl5qja4bq93JkqeslHrfLmlTHGXpdinjeiq45/hk4w15qv4f9dxPvRc+NAAAAAAAAAAAAAAAAAEzctvI55X9/Tf/tBACYm1+uMFbRmHgAAA=="
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 227,
      height = 34,
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
      compression = "gzip",
      data = "H4sIAAAAAAAACu3b3W2DMBSGYVDVdqPukRF603Qdbmg2yERt98gEsaogIeqfY7DxwX4f6ShEsZE58ImgKF0HAAAAAAAAAMf0VnoBAP58lF4AmnCble2z4VG2z1tx9CwOpRdQwC2iXPO29K3Fnm8xPirk6FkENBs7sgiUNnZkEShtmcOasrh8RpmeeeavOZ1NfSYcF2uv43TtfxBsS/flGusbPzq2Q5Y5iJk7mT9jS47JlsOaspiTJD9fpi6CfUnHQc6WJ8m1bcuBdO5arhy2nMWY+5MkP2SxDF+efNe3KwehubHfa+bjfTlsOYsxmSCLOknyZLvGfTkIzY09f9P4UA6l9/E9nHrKVjl6RO/dPSm9jhznlyym72uuc1X6GDVUTb0AAACo2VP//72rlq77LBFogi1jUmQRSIcsAjqQRUAHsgjoQBYBHcgioANZBHQgi4AO/NYP6LAmi9+mfkz97rlQoHJrsvhstl9MvfJ/ASCZ935bAUiDLLbpDiSy7fKYeAAA"
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 227,
      height = 34,
      id = 14,
      name = "ItemsInBlocks",
      visible = true,
      opacity = 1,
      offsetx = 0,
      offsety = 0,
      parallaxx = 1,
      parallaxy = 1,
      properties = {},
      encoding = "base64",
      compression = "gzip",
      data = "H4sIAAAAAAAACu3XMQpAIQgA0Dqq979ES2N8gobS/x4IgouKi60BAAAAAABAbjGDvPrtBpI6uXs7BwAAXrL6b+KjVtHNPy028ur+NCsAAAAAAAAAAAAAAAAAAAAAAAA1DASBRc2YeAAA"
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 227,
      height = 34,
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
      compression = "gzip",
      data = "H4sIAAAAAAAACu3VMQ0AAAgEMSzjf8HAMzPQJqfhqgAAAAAAAAAAAAAAAAAAAAAAAAAAAGDXIQD4KD3RGwH4yhcBAAAA4NYArJeBFJh4AAA="
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
          x = 1648,
          y = 320,
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
          x = 1824,
          y = 496,
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
