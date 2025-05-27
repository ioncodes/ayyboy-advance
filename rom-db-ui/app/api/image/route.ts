import { NextRequest } from 'next/server';
import { readFile } from 'fs/promises';
import path from 'path';

export async function GET(request: NextRequest) {
    const { searchParams } = new URL(request.url);
    const folder = searchParams.get('folder');
    const image = searchParams.get('image');
    
    if (!folder || !image) {
        return new Response('Missing folder or image parameter', { status: 400 });
    }
    
    try {
        const imagePath = path.resolve(process.cwd(), '../target/rom-db/', folder, image);
        const imageBuffer = await readFile(imagePath);
        
        return new Response(new Uint8Array(imageBuffer), {
            headers: {
                'Content-Type': 'image/png',
            }
        });
    } catch (error) {
        console.error('Error reading image:', error);
        return new Response('Image not found', { status: 404 });
    }
}
