import { promises as fs } from 'fs';
import path from 'path';

async function getScreenshotFolders() {
    try {
        const screenshotsPath = path.resolve(process.cwd(), '../target/rom-db/');

        try {
            await fs.access(screenshotsPath);
        } catch (error) {
            console.error('Screenshots directory not found:', error);
            return [];
        }

        const entries = await fs.readdir(screenshotsPath, { withFileTypes: true });

        return entries
            .filter(entry => entry.isDirectory())
            .map(dir => dir.name);

    } catch (error) {
        console.error('Error fetching screenshot folders:', error);
        return [];
    }
}

async function getImagesInFolder(folderName: string) {
    try {
        const folderPath = path.resolve(process.cwd(), '../target/rom-db/', folderName);
        const entries = await fs.readdir(folderPath);
        return entries.filter(file =>
            file.toLowerCase().endsWith('.png') ||
            file.toLowerCase().endsWith('.jpg') ||
            file.toLowerCase().endsWith('.jpeg')
        );
    } catch (error) {
        console.error(`Error reading images from folder ${folderName}:`, error);
        return [];
    }
}

export default async function ScreenshotsPage() {
    const folders = await getScreenshotFolders();

    const folderImages = await Promise.all(
        folders.map(async (folder) => {
            const images = await getImagesInFolder(folder);
            return { folder, images };
        })
    );

    return (
        <div className="space-y-8 p-4">
            {folders.length === 0 ? (
                <p>No screenshot folders found. Try running the ROM emulator first.</p>
            ) : (
                <div>
                    {folderImages.map(({ folder, images }) => (
                        <div key={folder} className="mb-6">
                            <h2 className="text-xl font-bold mb-2">{folder}</h2>

                            {images.length === 0 ? (
                                <p className="text-gray-500">No images found in this folder</p>
                            ) : (
                                <div className="flex flex-wrap gap-4">
                                    {images.map((image) => (
                                        <div key={image} className="border rounded overflow-hidden">
                                            <img
                                                src={`/api/image?folder=${encodeURIComponent(folder)}&image=${encodeURIComponent(image)}`}
                                                alt={`Screenshot from ${folder}`}
                                                className="max-h-48 object-contain"
                                            />
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
