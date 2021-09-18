return {
  version = "1.5",
  luaversion = "5.1",
  tiledversion = "1.7.2",
  orientation = "orthogonal",
  renderorder = "right-down",
  width = 228,
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
          id = 118,
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
          id = 119,
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
          id = 167,
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
      width = 228,
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
      chunks = {
        {
          x = -112, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgwA6yicS4wKj+Uf2j+snXT6wZlOoHAHBAn+sABAAA"
        },
        {
          x = -96, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = -80, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = -64, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = -48, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = -32, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = -16, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = 0, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = 16, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = 32, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = 48, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = 64, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = 80, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIHsWjeBSPSAwAlvuqagAEAAA="
        },
        {
          x = 96, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACstmYGDIJgLjAsToHdU/qn9U/+DUDwBOgfwoAAQAAA=="
        },
        {
          x = -16, y = 0, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgIA2EMaJiUsGo/lH9o/oHj34AU6HmIQAEAAA="
        },
        {
          x = 0, y = 0, width = 16, height = 16,
          data = "H4sIAAAAAAAACgtjZGAIIwGjA1L0juof1T+qf3DpBwBEEDBeAAQAAA=="
        }
      }
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 228,
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
      chunks = {
        {
          x = -112, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWjYHiD2RToPQXEv4D4DBl6f0L1/gbiP0D8lwJ30AoAANgzEwMABAAA"
        },
        {
          x = -96, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGPxgHhDPB+IFZOo/B8TngfgCtRw0CkYMmE2kun9A/B8JMzAyMJwCUr+A+Awe9QMNAJYGgFoABAAA"
        },
        {
          x = -80, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgoB+YB8TzoXgBDnF0OWRwDojPQ/EFHOLocqNgFNAUMA60AygDABXJsUwABAAA"
        },
        {
          x = -64, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGLpgHoX6z1HFFaNgqIDZFOg9BcS/gPgMFrl/QPwfihkYMeV/QvX+BuI/QPyXAndQGwAAip1FyAAEAAA="
        },
        {
          x = -48, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgoA+YB8TzgXgBmjg2MWzgHBCfB+ILaOLYxEbBKKAmmE2kun9A/B8JMzAyMJwCUr+A+AxtnEYxAACaPxAnAAQAAA=="
        },
        {
          x = -32, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGBgwD4jnE5CD4QVY1JwD4vM49MPkYPgCJQ4dBaOAAPgHxP9BDMYBdggZAACXK8zQAAQAAA=="
        },
        {
          x = -16, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgoAzMB+IFFOg/D8QXKHTDKBgFpILZFOg9BcS/gPgMHjX/gPg/FDMwIsR/QvX+BuI/FLiBWgAANHNTWAAEAAA="
        },
        {
          x = 0, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGJpgHhDPp0D/OSA+TyW3oAOY2xaQqR/mtgvUctAooCqYBcSzgXgOEP8loPYfEP9HwgyMDAyngNQvID5DQzcSCwDS/momAAQAAA=="
        },
        {
          x = 16, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGFiwgID8PCCej4TR1V8goP8cEJ9HwoTUj4JRQAn4B8T/QQzGAXYIkQAAY3OalQAEAAA="
        },
        {
          x = 32, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgoAzMA+L5ULyADP3ngPg8FF+g0C2jYBTQE5wiQs1/KEYHP4H4F3WdQxYAAAzyzhYABAAA"
        },
        {
          x = 48, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAW0APOAeD4QLyBT/zkgPg/EF6jloGEKZlOo/xcQn6HA7t9A/AeI/5KikZGB4RTU7oEGAMOon8wABAAA"
        },
        {
          x = 64, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGDgwD4rnA/ECMvSfg+LzQHyBes6iOoD5EYYXkKgf5kcYHsx+HWngDBaxf0D8H8RgpK9byAEAiz41ugAEAAA="
        },
        {
          x = 80, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgoAzMh+IFZOo/D8UXKHTHKBgFxIDZFOg9BcS/gPgMGXp/QvX+BuI/QPyXAndQEwAA+COWLQAEAAA="
        },
        {
          x = 96, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWjgLpAlRGByQGhQH2fgXQ4GfpBdqojYVLd8BmK1RgRbHL0hyHrJzMc6AEAikDIGwAEAAA="
        }
      }
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 228,
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
      chunks = {
        {
          x = -112, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWjYBQMNhDISBymVD8AkkdhOAAEAAA="
        },
        {
          x = -96, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAVDDUwcaAfQCaD78xNUDJke6iCQcWAxAHAJF0UABAAA"
        },
        {
          x = -80, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWjYOSBdCDOoEB+OhDPoIJ8IOPAYgBYoMtRAAQAAA=="
        },
        {
          x = -64, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWjYHCCdCDOoED/dCCeMcj1BzIOLAYAnwRBbgAEAAA="
        },
        {
          x = -48, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWjYGBBOhBnUKB/OhDPGKL6AxlJx9TUDwBa77MtAAQAAA=="
        },
        {
          x = -32, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAVDAXzCgYczAPlvIsPw9mcg48BiAEYWmIAABAAA"
        },
        {
          x = -16, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAUDCT5hwbjkJ9LXaaOADiCQEYJxiROLydUPAHl3mdkABAAA"
        },
        {
          x = 0, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAVDAUwcaAfQEHzCIT6RCPZQB4GMA4sBYLizRwAEAAA="
        },
        {
          x = 16, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAW0Ap+QMDa5ifR1zpAA2MKKGmoHKwhkHFgMAJm7HUMABAAA"
        },
        {
          x = 32, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWUgIlA/GmgHTGEAb6wm4aDTQyYhkU/sWZMY8Ctn5AZ03CoxSUeyDiwGACLVS2NAAQAAA=="
        },
        {
          x = 48, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWjYHCBaVCMzCdVL7p+YsyYxoBfPz4z0PXi0o9uRiAjbkyMGkr1AwBmSIG5AAQAAA=="
        },
        {
          x = 64, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWjYPCBT1A8EUrTEqQDcQYF6qcD8QwS9MPUTwPiQMaBxQDB4jnDAAQAAA=="
        },
        {
          x = 80, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWDEUyDYkr0kqN/GgP5+tH1kqIfm15i9ePSS4x+fHrpoT+QcWAxALYaDtwABAAA"
        },
        {
          x = 96, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGNrAjEL9aaP6R/VTAKZRqD+QkThMK/0A90jeWQAEAAA="
        },
        {
          x = -16, y = 0, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgIA0wMzIwLGdgcFgBxCRqhesvB+qtoEA/JWBU/6j+oaw/kREVU6ofADozN8IABAAA"
        },
        {
          x = 0, y = 0, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNmZGBgxoHRwXJMIZx66aWfFDCqf/jpp0f6uwrE14D4Opn6WYFsNiBmx+KmREbSMLX1AwCYgtCFAAQAAA=="
        }
      }
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 228,
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
      chunks = {
        {
          x = -96, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAVDDTAOtAPoBBLxiGGTGwWkAwD8nHmUAAQAAA=="
        },
        {
          x = -32, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWjYPACxoF2wDAHAM08rioABAAA"
        },
        {
          x = -16, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAVDBSQOtANGwbADAEwA77oABAAA"
        },
        {
          x = 0, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAVDATAOtAMGACQSwR4FlAEA5uG+SgAEAAA="
        },
        {
          x = 16, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAUDBRIH2gGjYMQDAJrqqBsABAAA"
        },
        {
          x = 32, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWUgMSBdsAoGAUUAABLdVABAAQAAA=="
        },
        {
          x = 64, y = -16, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAWjYPCCxIF2wDAHABQg8RYABAAA"
        }
      }
    },
    {
      type = "tilelayer",
      x = 0,
      y = 0,
      width = 228,
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
      chunks = {
        {
          x = 0, y = 0, width = 16, height = 16,
          data = "H4sIAAAAAAAACmNgGAUDDQqx4IG0m55uGGj7RzoAABrgkxgABAAA"
        }
      }
    }
  }
}
