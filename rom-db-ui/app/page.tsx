import { promises as fs } from 'fs';
import path from 'path';
import SearchComponent from './SearchComponent';

async function getScreenshotFolders() {
    try {
        const screenshotsPath = path.resolve(process.cwd(), 'screenshots/');

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
        const folderPath = path.resolve(process.cwd(), 'screenshots/', folderName);
        const entries = await fs.readdir(folderPath);
        return entries.filter(file => file.toLowerCase().endsWith('.png'));
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
            <SearchComponent folderData={folderImages} />
        </div>
    );
}
