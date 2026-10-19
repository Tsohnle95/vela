export default {
 name:'Following tilt',family:'Physics',description:'The nearby cube tips gently toward the pearl’s horizontal position.',rule:'A maximum 1.8-degree tilt follows direction and returns smoothly to rest.',
 setup({cubeGroups,set}){return ({i,x,y,nx,strength})=>set(cubeGroups[i],{transform:`rotate(${nx*strength*1.8} ${x} ${y})`});}
};
