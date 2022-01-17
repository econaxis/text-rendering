# Text Rendering Experiments

Experiments using the WebGPU API (which mimics Vulkan closely) to render proportional fonts.

![image](text.gif)

Subpixel Glyphs are rendered by the Freetype library onto a large texture atlas. Then, strings that the user wants to
display are deconstructed into rectangles and texture coordinates. These rectangles are then rendered using WebGPU.

There is absolutely no optimization done here. That means all data is sent to the GPU on each frame, even if the text
does not change. 

