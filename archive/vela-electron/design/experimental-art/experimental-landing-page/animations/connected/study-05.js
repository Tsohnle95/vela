export default {
 name:'Gentle attraction',family:'Physics',description:'Each cube leans slightly toward the passing pearl, then settles back.',rule:'Translation follows the pearl’s direction, capped at seven icon units.',
 setup({cubeGroups,set}){return ({i,nx,ny,strength})=>set(cubeGroups[i],{transform:`translate(${nx*strength*7} ${ny*strength*7})`});}
};
