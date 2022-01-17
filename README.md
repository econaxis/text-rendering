# Text Rendering Experiments

Experiments using the WebGPU API (which mimics Vulkan closely) to render proportional fonts.

![image](text.gif)

Subpixel Glyphs are rendered by the Freetype library onto a large texture atlas. Then, strings that the user wants to
display are deconstructed into rectangles and texture coordinates. These rectangles are then rendered using WebGPU.

There is absolutely no optimization done here. That means all data is sent to the GPU on each frame, even if the text
does not change. 


# Motivation

When debugging, I often wanted two, scrollable views to `stdout` that I could cross-reference. Having to scroll up/down large amounts of text was inconvenient and mentally overwhelming. 

This is a first-attempt solution to part of that problem -- text rendering. The side-by-side scrollable text views shows what my intended solution would be.
