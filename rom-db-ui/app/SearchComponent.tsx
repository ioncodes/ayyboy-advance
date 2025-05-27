'use client';

import { useState } from 'react';

type FolderData = {
    folder: string;
    images: string[];
};

export default function SearchComponent({ folderData }: { folderData: FolderData[] }) {
    const [searchTerm, setSearchTerm] = useState('');

    const filteredFolders = folderData.filter(item =>
        item.folder.toLowerCase().includes(searchTerm.toLowerCase())
    );

    return (
        <>
            <div className="mb-6">
                <input
                    type="text"
                    placeholder="Search folders..."
                    className="w-full p-2 border rounded"
                    value={searchTerm}
                    onChange={(e) => setSearchTerm(e.target.value)}
                />
            </div>

            {filteredFolders.length === 0 ? (
                <p>No matching folders found.</p>
            ) : (
                <div>
                    {filteredFolders.map(({ folder, images }) => (
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
        </>
    );
}
