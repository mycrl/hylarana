//
//  devices.c
//  devices
//
//  Created by Mr.Panda on 2024/2/14.
//

#include "devices.h"

int init(VideoInfo* info)
{
    if (obs_initialized())
    {
        return -1;
    }
    
    if (!obs_startup("en-US", NULL, NULL))
    {
        return -2;
    }

    struct obs_video_info video_info;
    video_info.graphics_module = "libobs-d3d11";
    video_info.fps_num = info->fps;
    video_info.fps_den = 1;
    video_info.gpu_conversion = true;
    video_info.base_width = info->width;
    video_info.base_height = info->height;
    video_info.output_width = info->width;
    video_info.output_height = info->height;
    video_info.output_format = info->format;
    video_info.colorspace = VIDEO_CS_DEFAULT;
    video_info.range = VIDEO_RANGE_DEFAULT;
    video_info.scale_type = OBS_SCALE_DISABLE;
    video_info.adapter = 0;

    if (obs_reset_video(&video_info) != OBS_VIDEO_SUCCESS)
    {
        return -3;
    }

    obs_load_all_modules();
	obs_post_load_modules();
    return 0;
}

void set_video_output_callback(VideoOutputCallback proc, void* ctx)
{
    obs_add_raw_video_callback(NULL, proc, ctx);
}

DeviceManager* create_device_manager()
{
    DeviceManager* manager = (DeviceManager*)malloc(sizeof(DeviceManager));
    if (manager == NULL)
    {
        return NULL;
    }

    manager->scene = obs_scene_create("mirror");
	if (manager->scene == NULL)
	{
        device_manager_release(manager);
		return NULL;
	}

	manager->video_source = obs_source_create("dshow_input", "mirror video input", NULL, NULL);
	if (manager->video_source == NULL)
	{
        device_manager_release(manager);
		return NULL;
	}
    else
    {
        obs_set_output_source(0, manager->video_source);
    }

	manager->video_scene_item = obs_scene_add(manager->scene, manager->video_source);
	if (manager->video_scene_item == NULL)
	{
        device_manager_release(manager);
		return NULL;
	}
	else
	{
		obs_sceneitem_set_visible(manager->video_scene_item, true);
	}

    return manager;
}

void device_manager_release(DeviceManager* manager)
{
    if (manager->scene != NULL)
    {
        obs_scene_release(manager->scene);
    }

    if (manager->video_source != NULL) 
    {
        obs_source_release(manager->video_source);
    }

    if (manager->video_scene_item != NULL)
    {
        obs_sceneitem_release(manager->video_scene_item);
    }

    free(manager);
}

void set_video_input(DeviceManager* manager, DeviceDescription* description, VideoInfo* info)
{
    obs_data_t* settings = obs_data_create();
    obs_data_t* cur_settings = obs_source_get_settings(manager->video_source);
    obs_data_apply(settings, cur_settings);

    char resolution[20];
    sprintf(resolution, "%dx%d", info->width, info->height);

    obs_data_set_int(settings, "res_type", 1);
    obs_data_set_bool(settings, "hw_decode", true);
    obs_data_set_string(settings, "resolution", &resolution);
    obs_data_set_string(settings, "video_device_id", description->id);
    obs_source_update(manager->video_source, settings);
    obs_data_release(settings);
}

DeviceList get_device_list(DeviceManager* manager, DeviceType type)
{
    DeviceList list;
    list.size = 0;
    list.devices = (DeviceDescription**)malloc(sizeof(DeviceDescription*) * 50);

    obs_properties_t* properties = obs_source_properties(manager->video_source);
	obs_property_t* property = obs_properties_first(properties);
	while (property)
	{
		const char* name = obs_property_name(property);
		if (strcmp(name, "video_device_id") == 0)
		{
			for (size_t i = 0; i < obs_property_list_item_count(property); i++)
			{
                DeviceDescription* device = (DeviceDescription*)malloc(sizeof(DeviceDescription));
                if (device != NULL)
                {
                    device->type = type;
                    device->id = obs_property_list_item_string(property, i);
                    device->name = obs_property_list_item_name(property, i);
                    list.devices[list.size] = device;
                    list.size ++;
                }
			}
		}

		obs_property_next(&property);
	}

    return list;
}

void release_device_description(DeviceDescription* description)
{
    free(description);
}